//! Preview-only rig mapping, socket placement, skin remap, and pose application.

use std::collections::{HashMap, HashSet};

use bevy::{mesh::skinning::SkinnedMesh, prelude::*};
use crozon_characters::{CharacterPartSlot, ResolvedRigPose};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterRigRole {
	Body,
	Head,
}

#[derive(Component)]
pub struct CharacterRig {
	pub role: CharacterRigRole,
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

/// Bind-pose bone scales captured once each named bone appears in the rig map.
#[derive(Component, Default)]
pub struct RigBindScales {
	pub scales: HashMap<String, Vec3>,
}

fn bone_map_ready(map: &BoneMap) -> bool {
	["root", "pelvis.L", "chest.L", "waist.L"]
		.iter()
		.all(|bone| map.by_name.contains_key(*bone))
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
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		// Rebuild each frame so bones that appear after the initial GLTF spawn are
		// included before pose maintenance runs.
		map.by_name.clear();

		let mut stack: Vec<Entity> = children.iter().collect();
		while let Some(entity) = stack.pop() {
			if let Ok(name) = names_q.get(entity) {
				map.by_name.insert(name.to_string(), entity);
			}
			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}
	}
}

pub fn attach_parts_to_sockets(
	mut commands: Commands,
	mut parts: Query<(Entity, &mut Transform, &NeedsSocketPlacement), With<CharacterPart>>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
) {
	for (entity, mut transform, placement) in &mut parts {
		let Ok(rig_map) = rig_maps.get(placement.rig_root) else {
			continue;
		};
		let Some(bone_entity) = rig_map.by_name.get(placement.socket_bone) else {
			continue;
		};

		let normalization_scale = transform.scale;
		*transform = placement.local_transform;
		transform.scale *= normalization_scale;

		commands.entity(entity).insert(ChildOf(*bone_entity));
		commands.entity(entity).remove::<NeedsSocketPlacement>();
	}
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
				} else {
					all_meshes_ok = false;
				}
			}

			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}

		if !any_skinned {
			commands.entity(part_root).remove::<NeedsSkinRemap>();
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
				.insert(NoMatchingArmature { missing_joints: missing });
		}

		commands.entity(part_root).remove::<NeedsSkinRemap>();
	}
}

pub fn maintain_resolved_pose(
	mut rig_roots: Query<(&BoneMap, &ActiveRigPose, &mut RigBindScales), With<CharacterRig>>,
	mut transforms: Query<&mut Transform>,
) {
	for (bone_map, active_pose, mut bind_scales) in &mut rig_roots {
		if !bone_map_ready(bone_map) {
			continue;
		}

		for (bone_name, entity) in &bone_map.by_name {
			if bind_scales.scales.contains_key(bone_name) {
				continue;
			}
			let Ok(transform) = transforms.get(*entity) else {
				continue;
			};
			bind_scales.scales.insert(bone_name.clone(), transform.scale);
		}

		for (bone_name, entity) in &bone_map.by_name {
			let multiplier = active_pose.pose.scale_for_bone(bone_name);
			if multiplier == Vec3::ONE {
				continue;
			}
			let Some(bind_scale) = bind_scales.scales.get(bone_name) else {
				continue;
			};
			let Ok(mut transform) = transforms.get_mut(*entity) else {
				continue;
			};
			transform.scale = *bind_scale * multiplier;
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
