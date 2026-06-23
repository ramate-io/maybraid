use std::collections::HashMap;

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
pub struct NeedsSkinRemap {
	pub rig_root: Entity,
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
	part_roots: Query<(Entity, &Children, &NeedsSkinRemap), With<ModularPart>>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	mut skinned_meshes: Query<&mut SkinnedMesh>,
) {
	for (part_root, children, remap) in &part_roots {
		let Ok(rig_map) = rig_maps.get(remap.rig_root) else {
			continue;
		};

		if rig_map.by_name.is_empty() {
			continue;
		}

		let mut stack: Vec<Entity> = children.iter().collect();
		let mut remapped_any = false;

		while let Some(entity) = stack.pop() {
			if let Ok(mut skin) = skinned_meshes.get_mut(entity) {
				let mut new_joints = Vec::with_capacity(skin.joints.len());

				for old_joint in &skin.joints {
					let Ok(old_name) = names_q.get(*old_joint) else {
						continue;
					};

					let Some(new_joint) = rig_map.by_name.get(old_name.as_str()) else {
						warn!("No matching rig joint for part joint {}", old_name);
						continue;
					};

					new_joints.push(*new_joint);
				}

				if new_joints.len() == skin.joints.len() {
					skin.joints = new_joints;
					remapped_any = true;
				} else {
					warn!("Part skin remap failed due to missing joints");
				}
			}

			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}

		if remapped_any {
			commands.entity(part_root).remove::<NeedsSkinRemap>();
		}
	}
}

pub fn attach_parts_to_sockets(
	mut commands: Commands,
	mut parts: Query<
		(Entity, &mut Transform, &NeedsSkinRemap, &NeedsSocketPlacement),
		With<ModularPart>,
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
) {
	for (entity, mut transform, remap, placement) in &mut parts {
		let Ok(rig_map) = rig_maps.get(remap.rig_root) else {
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
