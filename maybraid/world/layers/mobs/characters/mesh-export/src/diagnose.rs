//! Per-part transform / skinning diagnostics at export time.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{Children, ChildOf};
use bevy::ecs::relationship::RelationshipTarget;
use bevy::math::Vec3;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::{GlobalTransform, Mesh, Mesh3d, Name, Transform, World};
use characters::{
	member::CharacterMembers,
	nodes::PartNode,
	rig::{BoneMap, CharacterPart, CharacterRig},
	CharacterRoot,
};
use rigs::{SocketRefApplied, SocketRefRoot};
use serde::Serialize;

use crate::export::{part_group_name, part_mesh_entities, primary_mesh_under};
use crate::reference::{bbox, skin_mesh_world_reference};
use crate::skin::skin_weight_coverage;

#[derive(Serialize)]
pub struct TransformStep {
	pub entity: String,
	pub name: Option<String>,
	pub local_translation: [f32; 3],
	pub local_rotation: [f32; 4],
	pub local_scale: [f32; 3],
	pub global_translation: [f32; 3],
	pub global_scale: [f32; 3],
}

#[derive(Serialize)]
pub struct PartDiagnosis {
	pub group: String,
	pub part_host: String,
	pub part_slot: String,
	pub part_node: Option<PartNodeSnapshot>,
	pub socket: Option<SocketSnapshot>,
	pub mesh_entity: String,
	pub mesh_global: [f32; 3],
	pub mesh_local_scale: [f32; 3],
	pub has_skinned_mesh: bool,
	pub joint_count: usize,
	pub sample_bind_v0: [f32; 3],
	pub sample_skinned_world_v0: [f32; 3],
	pub sample_obj_character_v0: [f32; 3],
	pub reference_bbox_min: [f32; 3],
	pub reference_bbox_max: [f32; 3],
	pub obj_bbox_min: [f32; 3],
	pub obj_bbox_max: [f32; 3],
	pub bbox_delta_m: f32,
	pub bind_local_bbox_min: [f32; 3],
	pub bind_local_bbox_max: [f32; 3],
	pub mesh_world_scale: [f32; 3],
	pub live_weight_vertex_fraction: f32,
	pub first_divergence_step: String,
	pub chain_to_root: Vec<TransformStep>,
}

#[derive(Serialize)]
pub struct PartNodeSnapshot {
	pub label: String,
	pub normalization_scale: f32,
	pub feature_translation: [f32; 3],
	pub feature_scale: [f32; 3],
}

#[derive(Serialize)]
pub struct SocketSnapshot {
	pub bone: Option<String>,
	pub local_translation: [f32; 3],
	pub local_scale: [f32; 3],
	pub applied: bool,
}

#[derive(Serialize)]
pub struct DiagnoseReport {
	pub parts: Vec<PartDiagnosis>,
	pub divergence_summary: Option<String>,
}

fn entity_label(world: &World, entity: Entity) -> String {
	world
		.get::<Name>(entity)
		.map(|n| n.to_string())
		.unwrap_or_else(|| format!("{entity:?}"))
}

fn transform_step(world: &World, entity: Entity) -> TransformStep {
	let local = world.get::<Transform>(entity).cloned().unwrap_or_default();
	let global = world.get::<GlobalTransform>(entity).cloned().unwrap_or_default();
	let (s, _, _) = global.to_scale_rotation_translation();
	TransformStep {
		entity: format!("{entity:?}"),
		name: world.get::<Name>(entity).map(|n| n.to_string()),
		local_translation: local.translation.to_array(),
		local_rotation: local.rotation.to_array(),
		local_scale: local.scale.to_array(),
		global_translation: global.translation().to_array(),
		global_scale: s.to_array(),
	}
}

fn chain_to_character_root(world: &World, from: Entity, root: Entity) -> Vec<TransformStep> {
	let mut chain = Vec::new();
	let mut current = Some(from);
	while let Some(entity) = current {
		chain.push(transform_step(world, entity));
		if entity == root {
			break;
		}
		current = world.get::<ChildOf>(entity).map(|c| c.parent());
	}
	chain
}

pub fn build_diagnose_report(
	world: &mut World,
	root: Entity,
	obj_positions_by_group: &HashMap<String, Vec<Vec3>>,
) -> DiagnoseReport {
	let globals: Vec<(Entity, GlobalTransform)> = {
		let mut query = world.query::<(Entity, &GlobalTransform)>();
		query.iter(world).map(|(e, g)| (e, *g)).collect()
	};
	let global_refs: Vec<(Entity, &GlobalTransform)> =
		globals.iter().map(|(e, g)| (*e, g)).collect();
	let root_inverse = world
		.get::<GlobalTransform>(root)
		.map(|t| t.affine().inverse())
		.unwrap_or_default();

	let mut parts = Vec::new();
	let Some(members) = world.get::<CharacterMembers>(root) else {
		return DiagnoseReport { parts, divergence_summary: Some("no CharacterMembers".into()) };
	};

	for member in members.iter() {
		let Some(part) = world.get::<CharacterPart>(member) else {
			continue;
		};
		let mesh_entity = primary_mesh_under(world, member);
		let Some(mesh_entity) = mesh_entity else {
			continue;
		};

		let group = part_group_name(world, mesh_entity);
		let meshes = world.resource::<bevy::prelude::Assets<Mesh>>();
		let mesh_3d = world.get::<Mesh3d>(mesh_entity).unwrap();
		let mesh = meshes.get(&mesh_3d.0).unwrap();

		let bind_v0 = mesh_positions_v0(mesh);
		let mesh_global = world.get::<GlobalTransform>(mesh_entity).unwrap();
		let mesh_world = mesh_global.affine();
		let (mesh_world_scale, _, _) = mesh_global.to_scale_rotation_translation();

		let bind_local = mesh_positions(mesh);
		let (bind_min, bind_max) = bbox(&bind_local);

		let mut reference_world = Vec::new();
		let mut has_skin = false;
		let joint_count;
		let live_fraction = if let Some(skin) = world.get::<SkinnedMesh>(mesh_entity) {
			has_skin = true;
			joint_count = skin.joints.len();
			let bindposes = world
				.resource::<bevy::prelude::Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
				.get(&skin.inverse_bindposes)
				.unwrap();
			let coverage = skin_weight_coverage(mesh, skin, bindposes, &global_refs);
			reference_world =
				skin_mesh_world_reference(mesh, skin, bindposes, &global_refs, mesh_world)
				.unwrap_or_default();
			coverage.live_fraction
		} else {
			joint_count = 0;
			for p in &bind_local {
				reference_world.push(mesh_global.transform_point(*p));
			}
			1.0
		};

		let reference_character: Vec<Vec3> = reference_world
			.iter()
			.map(|p| root_inverse.transform_point3(*p))
			.collect();
		let (ref_min, ref_max) = bbox(&reference_character);

		let obj_positions = obj_positions_by_group.get(&group).cloned().unwrap_or_default();
		let (obj_min, obj_max) = bbox(&obj_positions);
		let delta = corner_delta(ref_min, ref_max, obj_min, obj_max);

		let part_node = world.get::<PartNode>(member).map(|n| PartNodeSnapshot {
			label: n.label.to_string(),
			normalization_scale: n.normalization.scale,
			feature_translation: n.feature.translation.to_array(),
			feature_scale: n.feature.scale.to_array(),
		});

		let socket = world.get::<SocketRefRoot>(member).map(|s| SocketSnapshot {
			bone: s.0.bone.map(str::to_string),
			local_translation: s.0.local.translation.to_array(),
			local_scale: s.0.local.scale.to_array(),
			applied: world.get::<SocketRefApplied>(member).is_some(),
		});

		let mesh_local = world.get::<Transform>(mesh_entity).cloned().unwrap_or_default();

		let first_divergence = divergence_step(
			&reference_character,
			&obj_positions,
			&bind_local,
			mesh_world,
			root_inverse,
		);

		parts.push(PartDiagnosis {
			group,
			part_host: entity_label(world, member),
			part_slot: format!("{:?}", part.slot),
			part_node,
			socket,
			mesh_entity: entity_label(world, mesh_entity),
			mesh_global: mesh_global.translation().to_array(),
			mesh_local_scale: mesh_local.scale.to_array(),
			has_skinned_mesh: has_skin,
			joint_count,
			sample_bind_v0: bind_v0.to_array(),
			sample_skinned_world_v0: reference_world.first().copied().unwrap_or(Vec3::ZERO).to_array(),
			sample_obj_character_v0: obj_positions.first().copied().unwrap_or(Vec3::ZERO).to_array(),
			reference_bbox_min: ref_min.to_array(),
			reference_bbox_max: ref_max.to_array(),
			obj_bbox_min: obj_min.to_array(),
			obj_bbox_max: obj_max.to_array(),
			bbox_delta_m: delta,
			bind_local_bbox_min: bind_min.to_array(),
			bind_local_bbox_max: bind_max.to_array(),
			mesh_world_scale: mesh_world_scale.to_array(),
			live_weight_vertex_fraction: live_fraction,
			first_divergence_step: first_divergence,
			chain_to_root: chain_to_character_root(world, mesh_entity, root),
		});
	}

	let divergence = parts
		.iter()
		.max_by(|a, b| a.bbox_delta_m.total_cmp(&b.bbox_delta_m))
		.filter(|p| p.bbox_delta_m > 1e-4)
		.map(|p| format!("first bbox divergence in group `{}` (Δ={:.6} m)", p.group, p.bbox_delta_m));

	DiagnoseReport { parts, divergence_summary: divergence }
}

fn is_under(world: &World, entity: Entity, ancestor: Entity) -> bool {
	let mut current = Some(entity);
	while let Some(e) = current {
		if e == ancestor {
			return true;
		}
		current = world.get::<ChildOf>(e).map(|c| c.parent());
	}
	false
}

fn mesh_positions(mesh: &Mesh) -> Vec<Vec3> {
	use bevy::mesh::VertexAttributeValues;
	match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
		Some(VertexAttributeValues::Float32x3(p)) => p.iter().map(|a| Vec3::from_array(*a)).collect(),
		_ => Vec::new(),
	}
}

fn mesh_positions_v0(mesh: &Mesh) -> Vec3 {
	mesh_positions(mesh).first().copied().unwrap_or(Vec3::ZERO)
}

fn corner_delta(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> f32 {
	a_min.distance(b_min).max(a_max.distance(b_max))
}

fn divergence_step(
	reference_character: &[Vec3],
	obj_character: &[Vec3],
	bind_local: &[Vec3],
	mesh_world: bevy::math::Affine3A,
	root_inverse: bevy::math::Affine3A,
) -> String {
	if reference_character.is_empty() || obj_character.is_empty() {
		return "missing positions".into();
	}
	let v0_ref = reference_character[0];
	let v0_obj = obj_character[0];
	if v0_ref.distance(v0_obj) <= 1e-4 {
		return "none (OBJ matches runtime reference skinning)".into();
	}
	let bind_char = root_inverse.transform_point3(mesh_world.transform_point3(bind_local[0]));
	if bind_char.distance(v0_obj) <= 1e-4 {
		return "OBJ used bind-local / mesh-world only (skinning skipped or zero live weight)".into();
	}
	let bind_raw_char = root_inverse.transform_point3(bind_local[0]);
	if bind_raw_char.distance(v0_obj) <= 1e-4 {
		return "OBJ used raw bind-local without mesh entity transform".into();
	}
	if v0_ref.distance(v0_obj) > 1e-4 {
		return "OBJ differs from reference skinning (post-skin transform step)".into();
	}
	"unknown".into()
}

pub fn write_diagnose_json(path: &Path, report: &DiagnoseReport) {
	let json = serde_json::to_string_pretty(report).expect("serialize diagnose");
	let mut file = File::create(path).expect("create diagnose json");
	file.write_all(json.as_bytes()).expect("write diagnose json");
}
