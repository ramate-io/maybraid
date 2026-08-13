//! Preview-only socket placement for hidden focus-reference rigs.
//!
//! Visible preview parts socket through [`crozon_characters::SocketRef`].
//! Proportion pose is [`crozon_characters::maintain_resolved_pose`].
//!
//! Rig/part markers, bone maps, and skin remap live in `crozon-characters`.

use bevy::prelude::*;

pub use crozon_characters::{
	bind_scales_ready, bone_map_ready, missing_landmark_bones, ActiveRigPose, BoneMap,
	CharacterPart, CharacterRig, CharacterRigRole, NeedsDuplicateScenePrune, NeedsSkinRemap,
	NoMatchingArmature, ResolvedPoseApplied, RigBindScales,
};

#[derive(Component)]
pub struct NeedsSocketPlacement {
	pub rig_root: Entity,
	pub socket_bone: &'static str,
	pub local_transform: Transform,
}

#[derive(Resource, Default)]
pub struct DumpBonesRequest(pub bool);

pub fn request_dump_bones(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		world.resource_mut::<DumpBonesRequest>().0 = true;
	});
}

pub fn preview_debug_enabled() -> bool {
	std::env::var("CROZON_PREVIEW_DEBUG").is_ok()
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

	if preview_debug_enabled() {
		info!(
			"Concept socket attach: {:?} → bone `{}` on {:?}",
			entity, placement.socket_bone, placement.rig_root
		);
	}

	commands.entity(entity).try_insert(ChildOf(*bone_entity));
	commands.entity(entity).try_remove::<NeedsSocketPlacement>();
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
