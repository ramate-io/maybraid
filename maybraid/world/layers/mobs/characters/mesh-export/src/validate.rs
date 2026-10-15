//! Runtime assembly metrics at export time (reference for OBJ re-import checks).

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bevy::ecs::entity::Entity;
use bevy::ecs::relationship::RelationshipTarget;
use bevy::math::Vec3;
use bevy::prelude::{GlobalTransform, World};
use characters::{
	member::CharacterMembers,
	rig::{BoneMap, CharacterRig},
};
use serde::Serialize;

use crate::export::{collect_groups, MeshGroup};

#[derive(Debug, Clone, Serialize)]
pub struct PartBounds {
	pub min: [f32; 3],
	pub max: [f32; 3],
}

#[derive(Debug, Serialize)]
pub struct SocketReport {
	pub bone: String,
	pub rig: String,
	pub position: [f32; 3],
}

#[derive(Debug, Serialize)]
pub struct ValidationReport {
	pub groups: usize,
	pub ecs_group_bounds: HashMap<String, PartBounds>,
	pub sockets: Vec<SocketReport>,
	pub combined_ecs_bbox: PartBounds,
}

fn bounds_from_positions(positions: &[Vec3]) -> PartBounds {
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in positions {
		min = min.min(*p);
		max = max.max(*p);
	}
	PartBounds { min: min.to_array(), max: max.to_array() }
}

pub fn build_validation_report(world: &mut World, root: Entity, groups: &[MeshGroup]) -> ValidationReport {
	let ecs_group_bounds = groups
		.iter()
		.map(|g| (g.name.clone(), bounds_from_positions(&g.positions)))
		.collect();
	let combined = bounds_from_positions(
		&groups.iter().flat_map(|g| g.positions.iter().copied()).collect::<Vec<_>>(),
	);
	ValidationReport {
		groups: groups.len(),
		ecs_group_bounds,
		sockets: collect_socket_positions(world, root),
		combined_ecs_bbox: combined,
	}
}

fn collect_socket_positions(world: &World, root: Entity) -> Vec<SocketReport> {
	let mut out = Vec::new();
	let Some(members) = world.get::<CharacterMembers>(root) else {
		return out;
	};
	let targets = [
		("body", "head_socket"),
		("body", "tailbone"),
		("head", "root"),
		("head", "eye_socket.L"),
		("head", "eye_socket.R"),
		("head", "nose_socket"),
		("head", "ear_socket.L"),
		("head", "ear_socket.R"),
	];
	for (rig_label, bone) in targets {
		if let Some(pos) = bone_world_position(world, members, rig_label, bone) {
			out.push(SocketReport {
				bone: bone.to_string(),
				rig: rig_label.to_string(),
				position: pos.to_array(),
			});
		}
	}
	out
}

fn bone_world_position(
	world: &World,
	members: &CharacterMembers,
	rig_role_label: &str,
	bone: &str,
) -> Option<Vec3> {
	use characters::rig::CharacterRigRole;

	let role = match rig_role_label {
		"body" => CharacterRigRole::Body,
		"head" => CharacterRigRole::Head,
		_ => return None,
	};

	for member in members.iter() {
		let Some(rig) = world.get::<CharacterRig>(member) else {
			continue;
		};
		if rig.role != role {
			continue;
		}
		let Some(map) = world.get::<BoneMap>(member) else {
			continue;
		};
		let Some(joint) = map.by_name.get(bone) else {
			continue;
		};
		return world.get::<GlobalTransform>(*joint).map(|t| t.translation());
	}
	None
}

pub fn write_validation_json(path: &Path, report: &ValidationReport) {
	let json = serde_json::to_string_pretty(report).expect("serialize validation");
	let mut file = File::create(path).expect("create validation json");
	file.write_all(json.as_bytes()).expect("write validation json");
}
