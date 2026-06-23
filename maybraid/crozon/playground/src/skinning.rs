use std::collections::{HashMap, HashSet};

use bevy::{mesh::skinning::SkinnedMesh, prelude::*};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum ModularPartKind {
	Body,
	Head,
	Mouth,
	Nose,
}

#[derive(Component)]
pub struct NeedsSocketPlacement {
	pub socket_bone: &'static str,
	pub scale: f32,
}

pub const HEAD_SOCKET_BONE: &str = "upper_neck";
pub const HEAD_SCALE: f32 = 0.15;

#[derive(Component)]
pub struct CharacterRig;

#[derive(Component)]
pub struct ModularPart;

#[derive(Component)]
pub struct PartRigRef {
	pub rig_root: Entity,
}

#[derive(Component)]
pub struct NeedsSkinRemap;

/// Part mesh was skinned to a skeleton that does not match the active rig (e.g. head armature).
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

pub fn build_rig_bone_map(
	mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		if !map.by_name.is_empty() {
			continue;
		}

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

pub fn remap_part_skin_to_rig(
	mut commands: Commands,
	part_roots: Query<
		(Entity, &Children, &PartRigRef, &NeedsSkinRemap),
		(With<ModularPart>, Without<NoMatchingArmature>),
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	mut skinned_meshes: Query<&mut SkinnedMesh>,
) {
	for (part_root, children, rig_ref, _needs_remap) in &part_roots {
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
			continue;
		}

		if !all_meshes_ok {
			let mut missing: Vec<_> = missing_joints.into_iter().collect();
			missing.sort();
			warn!(
				"Part skin remap failed ({} missing rig joints): {}",
				missing.len(),
				missing.join(", ")
			);
			commands.entity(part_root).insert(NoMatchingArmature { missing_joints: missing });
		}

		commands.entity(part_root).remove::<NeedsSkinRemap>();
	}
}

pub fn attach_parts_to_sockets(
	mut commands: Commands,
	mut parts: Query<
		(Entity, &mut Transform, &PartRigRef, &NeedsSocketPlacement),
		With<ModularPart>,
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
) {
	for (entity, mut transform, rig_ref, placement) in &mut parts {
		let Ok(rig_map) = rig_maps.get(rig_ref.rig_root) else {
			continue;
		};

		let Some(bone_entity) = rig_map.by_name.get(placement.socket_bone) else {
			continue;
		};

		commands.entity(entity).insert(ChildOf(*bone_entity));
		*transform = Transform::from_scale(Vec3::splat(placement.scale));

		commands.entity(entity).remove::<NeedsSocketPlacement>();
	}
}

pub fn dump_bones_to_console(
	mut request: ResMut<DumpBonesRequest>,
	mut console: ResMut<game_commands::command::CommandConsoleOutput>,
	rig_roots: Query<(Entity, &Children, &BoneMap), With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
) {
	if !request.0 {
		return;
	}
	request.0 = false;

	let Ok((_rig_root, children, bone_map)) = rig_roots.single() else {
		console.0 = "No character rig spawned.".into();
		return;
	};

	if bone_map.by_name.is_empty() {
		console.0 = "Rig bone map is empty (scene may still be loading).".into();
		return;
	}

	let mut output = String::from("Rig bone hierarchy:\n");

	for child in children.iter() {
		append_bone_tree(child, 0, &mut output, &children_q, &names_q);
	}

	if output.lines().count() <= 1 {
		output.push_str("(no named bones under rig root)\n");
		for name in bone_map.by_name.keys() {
			output.push_str(&format!("  {name}\n"));
		}
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
