//! Preview-only rig mapping, socket placement, skin remap, and pose application.

use std::collections::{HashMap, HashSet};

use bevy::{
	mesh::skinning::SkinnedMesh,
	prelude::*,
	world_serialization::{WorldInstance, WorldInstanceSpawner},
};
use crozon_characters::CharacterPartSlot;
use crozon_rigs::ResolvedRigPose;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RigSkeletonKind {
	#[default]
	Humanoid,
	Quadruped,
	/// Multi-bone neck armature (`neck_base` … `head_socket`).
	Neck,
}

impl RigSkeletonKind {
	pub fn from_body_rig_label(label: &str) -> Self {
		if label == "Quadruped" {
			Self::Quadruped
		} else {
			Self::Humanoid
		}
	}

	pub fn landmark_bones(self) -> &'static [&'static str] {
		match self {
			Self::Humanoid => &["root", "pelvis.L", "chest.L", "waist.L"],
			Self::Quadruped => &["head_socket", "shoulder.L", "tailbone", "waist.L"],
			Self::Neck => &["neck_base", "head_socket"],
		}
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterRigRole {
	Body,
	Neck,
	Head,
}

#[derive(Component)]
pub struct CharacterRig {
	pub role: CharacterRigRole,
	pub skeleton: RigSkeletonKind,
}

#[derive(Component, Debug)]
pub struct CharacterPart {
	pub slot: CharacterPartSlot,
}

#[derive(Component)]
pub struct PartRigRef {
	pub rig_root: Entity,
}

#[derive(Component)]
pub struct NeedsSkinRemap;

#[derive(Component)]
pub struct NeedsDuplicateScenePrune {
	pub keep: Vec<Entity>,
}

#[derive(Component)]
pub struct NeedsSocketPlacement {
	pub rig_root: Entity,
	pub socket_bone: &'static str,
	pub local_transform: Transform,
}

/// Resolved proportional layers to maintain on this rig across frames.
///
/// GLTF scene loads can reset bone transforms after the first application, so the
/// preview reapplies pose from a captured bind snapshot every frame.
#[derive(Component)]
pub struct ActiveRigPose {
	pub pose: ResolvedRigPose,
}

/// Bind-pose bone TRS captured once each named bone appears in the rig map.
#[derive(Component, Default)]
pub struct RigBindScales {
	pub scales: HashMap<String, Vec3>,
	pub translations: HashMap<String, Vec3>,
	pub rotations: HashMap<String, Quat>,
}

/// Inserted by [`maintain_resolved_pose`] the first frame it applies
/// [`ActiveRigPose`] to a rig whose landmark bones are all mapped.
///
/// This is the imperative "proportions are live" signal. Downstream consumers
/// (notably [`crate::camera_focus`]) should gate on this marker instead of
/// re-deriving readiness from bone transforms. It never needs clearing: pose
/// maintenance re-applies every frame, and rig respawns start from fresh
/// entities without the marker.
#[derive(Component)]
pub struct ResolvedPoseApplied;

pub fn preview_debug_enabled() -> bool {
	std::env::var("CROZON_PREVIEW_DEBUG").is_ok()
}

pub fn bone_map_ready(map: &BoneMap, skeleton: RigSkeletonKind) -> bool {
	skeleton.landmark_bones().iter().all(|bone| map.by_name.contains_key(*bone))
}

pub fn missing_landmark_bones(map: &BoneMap, skeleton: RigSkeletonKind) -> Vec<&'static str> {
	skeleton
		.landmark_bones()
		.iter()
		.copied()
		.filter(|bone| !map.by_name.contains_key(*bone))
		.collect()
}

/// Part mesh was skinned to a skeleton that does not match the active rig.
#[derive(Component, Debug)]
pub struct NoMatchingArmature {
	pub missing_joints: Vec<String>,
}

#[derive(Component, Default)]
pub struct BoneMap {
	pub by_name: HashMap<String, Entity>,
}

#[derive(Resource, Default)]
pub struct DumpBonesRequest(pub bool);

pub fn request_dump_bones(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		world.resource_mut::<DumpBonesRequest>().0 = true;
	});
}

pub fn build_rig_bone_map(
	mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	boundaries: Query<(), Or<(With<CharacterRig>, With<CharacterPart>)>>,
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		// Rebuild each frame so bones that appear after the initial GLTF spawn are
		// included before pose maintenance runs.
		map.by_name.clear();

		let mut stack: Vec<Entity> = children.iter().collect();
		while let Some(entity) = stack.pop() {
			// Socket-attached parts and nested rigs live under this rig's bones but
			// carry their own (often name-colliding) armatures; keep the map scoped
			// to this rig's skeleton only.
			if boundaries.contains(entity) {
				continue;
			}
			if let Ok(name) = names_q.get(entity) {
				map.by_name.insert(name.to_string(), entity);
			}
			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}
	}
}

pub fn attach_focus_reference_to_sockets(
	mut commands: Commands,
	mut parts: Query<
		(Entity, &mut Transform, &NeedsSocketPlacement),
		(With<crate::focus_reference::FocusReferenceRig>, Without<CharacterPart>),
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
) {
	for (entity, mut transform, placement) in &mut parts {
		attach_part_to_socket(&mut commands, entity, &mut transform, placement, &rig_maps);
	}
}

pub fn attach_parts_to_sockets(
	mut commands: Commands,
	mut parts: Query<(Entity, &mut Transform, &NeedsSocketPlacement), With<CharacterPart>>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
) {
	for (entity, mut transform, placement) in &mut parts {
		attach_part_to_socket(&mut commands, entity, &mut transform, placement, &rig_maps);
	}
}

fn attach_part_to_socket(
	commands: &mut Commands,
	entity: Entity,
	transform: &mut Transform,
	placement: &NeedsSocketPlacement,
	rig_maps: &Query<&BoneMap, With<CharacterRig>>,
) {
	let Ok(rig_map) = rig_maps.get(placement.rig_root) else {
		if preview_debug_enabled() {
			warn!(
				"Concept socket attach: rig root {:?} has no bone map yet (bone={})",
				placement.rig_root, placement.socket_bone
			);
		}
		return;
	};
	let Some(bone_entity) = rig_map.by_name.get(placement.socket_bone) else {
		if preview_debug_enabled() {
			warn!(
				"Concept socket attach: bone `{}` not found on rig {:?} (known bones={})",
				placement.socket_bone,
				placement.rig_root,
				rig_map.by_name.len()
			);
		}
		return;
	};

	let authored_scale = transform.scale;
	let authored_rotation = transform.rotation;
	*transform = placement.local_transform;
	// Authored asset scale and feature rotation are preserved; socket offset is applied first.
	transform.scale *= authored_scale;
	transform.rotation *= authored_rotation;

	commands.entity(entity).try_insert(ChildOf(*bone_entity));
	commands.entity(entity).try_remove::<NeedsSocketPlacement>();
}

pub fn remap_part_skin_to_rig(
	mut commands: Commands,
	part_roots: Query<
		(Entity, &Children, &CharacterPart, &PartRigRef, &NeedsSkinRemap),
		(With<CharacterPart>, Without<NoMatchingArmature>),
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	mut skinned_meshes: Query<&mut SkinnedMesh>,
	scene_instances: Query<&WorldInstance>,
	scene_spawner: Res<WorldInstanceSpawner>,
) {
	for (part_root, children, part, rig_ref, _needs_remap) in &part_roots {
		let Ok(rig_map) = rig_maps.get(rig_ref.rig_root) else {
			continue;
		};
		if rig_map.by_name.is_empty() {
			continue;
		}

		let mut stack: Vec<Entity> = children.iter().collect();
		let mut any_skinned = false;
		let mut all_meshes_ok = true;
		let mut missing_joints = HashSet::new();
		let mut remapped_meshes = Vec::new();

		while let Some(entity) = stack.pop() {
			if let Ok(mut skin) = skinned_meshes.get_mut(entity) {
				any_skinned = true;
				let mut new_joints = Vec::with_capacity(skin.joints.len());
				let mut mesh_ok = true;

				for old_joint in &skin.joints {
					let Ok(old_name) = names_q.get(*old_joint) else {
						mesh_ok = false;
						continue;
					};

					// Joint names must match between part armature and target rig.
					match rig_map.by_name.get(old_name.as_str()) {
						Some(new_joint) => new_joints.push(*new_joint),
						None => {
							missing_joints.insert(old_name.to_string());
							mesh_ok = false;
						}
					}
				}

				if mesh_ok && new_joints.len() == skin.joints.len() {
					skin.joints = new_joints;
					remapped_meshes.push(entity);
				} else {
					all_meshes_ok = false;
				}
			}

			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}

		if !any_skinned {
			let Ok(instance) = scene_instances.get(part_root) else {
				continue;
			};
			if !scene_spawner.instance_is_ready(**instance) {
				continue;
			}
			commands.entity(part_root).try_remove::<NeedsSkinRemap>();
			continue;
		}

		if !all_meshes_ok {
			let mut missing: Vec<_> = missing_joints.into_iter().collect();
			missing.sort();
			warn!(
				"Concept part {:?} skin remap failed ({} missing rig joints): {}",
				part.slot,
				missing.len(),
				missing.join(", ")
			);
			commands
				.entity(part_root)
				.try_insert(NoMatchingArmature { missing_joints: missing });
		} else {
			for entity in &remapped_meshes {
				// Clean clothing path: the mesh keeps its inverse bind poses, but
				// points at the live character rig joints by name.
				commands.entity(*entity).try_insert(ChildOf(part_root));
			}
			commands
				.entity(part_root)
				.try_insert(NeedsDuplicateScenePrune { keep: remapped_meshes });
		}

		commands.entity(part_root).try_remove::<NeedsSkinRemap>();
	}
}

pub fn prune_duplicate_part_scenes(
	mut commands: Commands,
	part_roots: Query<(Entity, Option<&Children>, &NeedsDuplicateScenePrune), With<CharacterPart>>,
) {
	for (part_root, children, prune) in &part_roots {
		let keep: HashSet<_> = prune.keep.iter().copied().collect();
		if let Some(children) = children {
			for child in children.iter() {
				if !keep.contains(&child) {
					// Whole-scene loading brings along the clothing file's duplicate
					// armature. Once remapped meshes are direct children of the part
					// root, the remaining scene hierarchy is no longer needed.
					commands.entity(child).try_despawn();
				}
			}
		}
		commands.entity(part_root).try_remove::<NeedsDuplicateScenePrune>();
	}
}

/// True once landmark bind scales have been captured for a rig.
pub fn bind_scales_ready(
	bind_scales: &RigBindScales,
	bone_map: &BoneMap,
	skeleton: RigSkeletonKind,
) -> bool {
	skeleton
		.landmark_bones()
		.iter()
		.all(|bone| bind_scales.scales.contains_key(*bone) && bone_map.by_name.contains_key(*bone))
}

pub fn maintain_resolved_pose(
	mut commands: Commands,
	mut rig_roots: Query<
		(
			Entity,
			&BoneMap,
			&ActiveRigPose,
			&mut RigBindScales,
			&CharacterRig,
			Has<ResolvedPoseApplied>,
		),
		With<CharacterRig>,
	>,
	mut transforms: Query<&mut Transform>,
	limb_animators: Query<(), With<crate::animation::LimbAnimator>>,
) {
	for (entity, bone_map, active_pose, mut bind_scales, rig, pose_applied) in &mut rig_roots {
		if !bone_map_ready(bone_map, rig.skeleton) {
			continue;
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			if bind_scales.scales.contains_key(bone_name)
				&& bind_scales.translations.contains_key(bone_name)
				&& bind_scales.rotations.contains_key(bone_name)
			{
				continue;
			}
			let Ok(transform) = transforms.get(*bone_entity) else {
				continue;
			};
			// First sighting after load: treat current transform as bind snapshot.
			bind_scales.scales.entry(bone_name.clone()).or_insert(transform.scale);
			bind_scales
				.translations
				.entry(bone_name.clone())
				.or_insert(transform.translation);
			bind_scales.rotations.entry(bone_name.clone()).or_insert(transform.rotation);
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			let scale_mult = active_pose.pose.scale_for_bone(bone_name);
			let trans_mult = active_pose.pose.translation_for_bone(bone_name);
			let rot_offset = active_pose.pose.rotation_for_bone(bone_name);
			if scale_mult == Vec3::ONE && trans_mult == Vec3::ONE && rot_offset == Quat::IDENTITY {
				continue;
			}
			let Ok(mut transform) = transforms.get_mut(*bone_entity) else {
				continue;
			};
			// Reapply every frame because GLTF spawn can reset bone transforms.
			if scale_mult != Vec3::ONE {
				if let Some(bind_scale) = bind_scales.scales.get(bone_name) {
					transform.scale = *bind_scale * scale_mult;
				}
			}
			if trans_mult != Vec3::ONE {
				if let Some(bind_trans) = bind_scales.translations.get(bone_name) {
					transform.translation = *bind_trans * trans_mult;
				}
			}
			// Skip bones owned by limb animation once their rest has been
			// captured (after the first pre-animator apply of this pitch).
			if rot_offset != Quat::IDENTITY && !limb_animators.contains(*bone_entity) {
				if let Some(bind_rot) = bind_scales.rotations.get(bone_name) {
					// Parent-space: `delta * bind`. Needed so `-neck_pitch` on
					// non-identity-rest `head_socket` cancels neck pitch.
					transform.rotation = rot_offset * *bind_rot;
				}
			}
		}

		// Landmarks mapped, bind scales snapshotted, pose written: proportions
		// are now live for this rig. Signal downstream consumers imperatively.
		if !pose_applied && bind_scales_ready(&bind_scales, bone_map, rig.skeleton) {
			commands.entity(entity).try_insert(ResolvedPoseApplied);
		}
	}
}

pub fn dump_bones_to_console(
	mut request: ResMut<DumpBonesRequest>,
	mut console: ResMut<game_commands::command::CommandConsoleOutput>,
	rig_roots: Query<(Entity, &Children, &BoneMap, &CharacterRig)>,
	failed_parts: Query<(&CharacterPart, &NoMatchingArmature)>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
) {
	if !request.0 {
		return;
	}
	request.0 = false;

	let mut output = String::from("Concept rig bone hierarchies:\n");
	let mut any = false;
	for (_rig_root, children, bone_map, rig) in &rig_roots {
		any = true;
		output.push_str(&format!("--- {:?} rig ---\n", rig.role));
		if bone_map.by_name.is_empty() {
			output.push_str("(bone map empty; scene may still be loading)\n");
			continue;
		}
		for child in children.iter() {
			append_bone_tree(child, 0, &mut output, &children_q, &names_q);
		}
	}

	if !any {
		output.push_str("No concept rigs spawned.\n");
	}

	for (part, failure) in &failed_parts {
		output.push_str(&format!(
			"Part {:?} missing joints: {}\n",
			part.slot,
			failure.missing_joints.join(", ")
		));
	}

	console.0 = output;
}

fn append_bone_tree(
	entity: Entity,
	indent: usize,
	output: &mut String,
	children_q: &Query<&Children>,
	names_q: &Query<&Name>,
) {
	if let Ok(name) = names_q.get(entity) {
		output.push_str(&format!("{:indent$}{name}\n", "", indent = indent * 2));
	}
	if let Ok(children) = children_q.get(entity) {
		for child in children.iter() {
			append_bone_tree(child, indent + 1, output, children_q, names_q);
		}
	}
}
