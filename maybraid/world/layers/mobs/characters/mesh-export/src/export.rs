//! Wavefront OBJ writer for assembled character meshes.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use bevy::asset::Assets;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{Children, ChildOf};
use bevy::ecs::relationship::RelationshipTarget;
use bevy::math::{Affine3A, Vec2, Vec3};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh, Mesh3d, Name};
use bevy::render::mesh::Indices;
use characters::{
	assembly::CharacterPartSlot,
	member::CharacterMembers,
	nodes::PartNode,
	rig::{BoneMap, CharacterPart, CharacterRig, NeedsDuplicateScenePrune, NeedsSkinRemap},
	CharacterRoot,
};
use rigs::{SocketRefApplied, SocketRefRoot};
use material_ref::MaterialRefRoot;

use crate::skin::skin_mesh_world;

#[derive(Debug, Clone)]
pub struct MeshGroup {
	pub name: String,
	pub material_name: String,
	pub diffuse: [f32; 3],
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub uvs: Vec<Vec2>,
	pub faces: Vec<[u32; 3]>,
}

#[derive(Debug)]
pub struct ExportReport {
	pub groups: usize,
	pub vertices: usize,
	pub faces: usize,
	pub has_uvs: bool,
	pub has_normals: bool,
	pub bounds_min: Vec3,
	pub bounds_max: Vec3,
	pub obj_bytes: u64,
	pub mtl_bytes: u64,
}

impl Default for ExportReport {
	fn default() -> Self {
		Self {
			groups: 0,
			vertices: 0,
			faces: 0,
			has_uvs: false,
			has_normals: false,
			bounds_min: Vec3::splat(f32::MAX),
			bounds_max: Vec3::splat(f32::MIN),
			obj_bytes: 0,
			mtl_bytes: 0,
		}
	}
}

pub fn collect_groups(world: &mut bevy::prelude::World, root: Entity) -> Vec<MeshGroup> {
	let mut groups: HashMap<String, MeshGroup> = HashMap::new();
	let globals: Vec<(Entity, GlobalTransform)> = world
		.query::<(Entity, &GlobalTransform)>()
		.iter(world)
		.map(|(e, g)| (e, *g))
		.collect();
	let global_refs: Vec<(Entity, &GlobalTransform)> =
		globals.iter().map(|(e, g)| (*e, g)).collect();

	let root_inverse = world
		.get::<GlobalTransform>(root)
		.map(|t| t.affine().inverse())
		.unwrap_or(Affine3A::IDENTITY);

	let mesh_entities: Vec<Entity> = part_mesh_entities(world, root);

	for mesh_entity in mesh_entities {
		let Some(mesh_3d) = world.get::<Mesh3d>(mesh_entity) else {
			continue;
		};
		let mesh_handle = mesh_3d.0.clone();
		let Some(mesh) = world.resource::<Assets<Mesh>>().get(&mesh_handle) else {
			continue;
		};
		let group_name = part_group_name(world, mesh_entity);
		let (diffuse, material_name) = part_material(world, mesh_entity, &group_name);

		let (positions, normals, uvs, faces) =
			extract_mesh_local(mesh, mesh_entity, world, &global_refs, root_inverse);

		let entry = groups.entry(group_name.clone()).or_insert_with(|| MeshGroup {
			name: group_name,
			material_name,
			diffuse,
			positions: Vec::new(),
			normals: Vec::new(),
			uvs: Vec::new(),
			faces: Vec::new(),
		});

		let base = entry.positions.len() as u32;
		entry.positions.extend(positions);
		entry.normals.extend(normals);
		entry.uvs.extend(uvs);
		entry.faces.extend(faces.into_iter().map(|[a, b, c]| [a + base, b + base, c + base]));
	}

	groups.into_values().collect()
}

pub(crate) fn part_group_name(world: &bevy::prelude::World, entity: Entity) -> String {
	let mut current = Some(entity);
	while let Some(e) = current {
		if let Some(part) = world.get::<PartNode>(e) {
			return format!("{}__{}", slot_label(part.slot), part.label);
		}
		if let Some(part) = world.get::<CharacterPart>(e) {
			return slot_label(part.slot).to_string();
		}
		current = world.get::<ChildOf>(e).map(|c| c.parent());
	}
	world
		.get::<Name>(entity)
		.map(|n| n.to_string())
		.unwrap_or_else(|| "mesh".to_string())
}

fn slot_label(slot: CharacterPartSlot) -> &'static str {
	match slot {
		CharacterPartSlot::BodyMesh => "body",
		CharacterPartSlot::HeadMesh => "head",
		CharacterPartSlot::EyeLeft => "eye_left",
		CharacterPartSlot::EyeRight => "eye_right",
		CharacterPartSlot::EarLeft => "ear_left",
		CharacterPartSlot::EarRight => "ear_right",
		CharacterPartSlot::Nose => "nose",
		CharacterPartSlot::Tail => "tail",
		CharacterPartSlot::Mouth => "mouth",
		_ => "part",
	}
}

fn part_material(
	world: &bevy::prelude::World,
	entity: Entity,
	fallback: &str,
) -> ([f32; 3], String) {
	let mut current = Some(entity);
	while let Some(e) = current {
		if let Some(root) = world.get::<MaterialRefRoot>(e) {
			let color = root.0.palette.first().copied().unwrap_or_default();
			let s = color.to_srgba();
			let mat_name = format!("{}_mat", fallback.replace('/', "_"));
			return ([s.red, s.green, s.blue], mat_name);
		}
		current = world.get::<ChildOf>(e).map(|c| c.parent());
	}
	([0.75, 0.75, 0.75], format!("{}_mat", fallback.replace('/', "_")))
}

/// One primary render mesh per [`CharacterPart`] host (largest `Mesh3d` under the host).
pub(crate) fn part_mesh_entities(world: &bevy::prelude::World, root: Entity) -> Vec<Entity> {
	let Some(members) = world.get::<CharacterMembers>(root) else {
		return Vec::new();
	};
	let mut meshes = Vec::new();
	for member in members.iter() {
		if world.get::<CharacterPart>(member).is_none() {
			continue;
		}
		if let Some(entity) = primary_mesh_under(world, member) {
			meshes.push(entity);
		}
	}
	meshes
}

fn mesh_vertex_count(world: &bevy::prelude::World, entity: Entity) -> usize {
	let Some(mesh_3d) = world.get::<Mesh3d>(entity) else {
		return 0;
	};
	let Some(mesh) = world.resource::<Assets<Mesh>>().get(&mesh_3d.0) else {
		return 0;
	};
	mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		.map(|a| a.len())
		.unwrap_or(0)
}

/// Largest mesh under `root` (skips rig helper curves / duplicate armature junk).
pub(crate) fn primary_mesh_under(world: &bevy::prelude::World, root: Entity) -> Option<Entity> {
	let mut stack = vec![root];
	let mut skinned_best: Option<(Entity, usize)> = None;
	let mut rigid_best: Option<(Entity, usize)> = None;
	while let Some(entity) = stack.pop() {
		if world.get::<Mesh3d>(entity).is_some() {
			let count = mesh_vertex_count(world, entity);
			if count >= 3 {
				let slot = if world.get::<SkinnedMesh>(entity).is_some() {
					&mut skinned_best
				} else {
					&mut rigid_best
				};
				if slot.is_none_or(|(_, c)| count > c) {
					*slot = Some((entity, count));
				}
			}
		}
		if let Some(children) = world.get::<Children>(entity) {
			stack.extend(children.iter());
		}
	}
	skinned_best.or(rigid_best).map(|(e, _)| e)
}

fn extract_mesh_local(
	mesh: &Mesh,
	entity: Entity,
	world: &bevy::prelude::World,
	globals: &[(Entity, &GlobalTransform)],
	root_inverse: Affine3A,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec2>, Vec<[u32; 3]>) {
	let uvs = mesh_uvs(mesh);
	let faces = mesh_faces(mesh);

	let mesh_world = world
		.get::<GlobalTransform>(entity)
		.map(|t| t.affine())
		.unwrap_or(Affine3A::IDENTITY);

	let (positions, normals) = if let Some(skin) = world.get::<SkinnedMesh>(entity) {
		let bind = world
			.resource::<Assets<SkinnedMeshInverseBindposes>>()
			.get(&skin.inverse_bindposes)
			.expect("bindposes loaded");
		let skinned = skin_mesh_world(mesh, skin, bind, globals, mesh_world).expect("skin mesh");
		let positions = skinned
			.positions
			.iter()
			.map(|p| root_inverse.transform_point3(*p))
			.collect();
		let normals = skinned
			.normals
			.iter()
			.map(|n| root_inverse.transform_vector3(*n).normalize())
			.collect();
		(positions, normals)
	} else {
		let world_tf = world.get::<GlobalTransform>(entity).expect("global");
		let local = root_inverse * world_tf.affine();
		let positions = mesh_positions(mesh)
			.iter()
			.map(|p| local.transform_point3(*p))
			.collect();
		let normals = mesh_normals(mesh)
			.iter()
			.map(|n| local.transform_vector3(*n).normalize())
			.collect();
		(positions, normals)
	};

	(positions, normals, uvs, faces)
}

fn mesh_positions(mesh: &Mesh) -> Vec<Vec3> {
	match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
		Some(VertexAttributeValues::Float32x3(p)) => {
			p.iter().map(|a| Vec3::from_array(*a)).collect()
		}
		_ => Vec::new(),
	}
}

fn mesh_normals(mesh: &Mesh) -> Vec<Vec3> {
	match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
		Some(VertexAttributeValues::Float32x3(n)) => {
			n.iter().map(|a| Vec3::from_array(*a)).collect()
		}
		_ => Vec::new(),
	}
}

fn mesh_uvs(mesh: &Mesh) -> Vec<Vec2> {
	match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
		Some(VertexAttributeValues::Float32x2(uv)) => {
			uv.iter().map(|a| Vec2::from_array(*a)).collect()
		}
		_ => Vec::new(),
	}
}

fn mesh_faces(mesh: &Mesh) -> Vec<[u32; 3]> {
	let indices = match mesh.indices() {
		Some(Indices::U32(i)) => i.clone(),
		Some(Indices::U16(i)) => i.iter().map(|&x| x as u32).collect(),
		None => {
			let n = mesh_positions(mesh).len();
			(0..n as u32).collect()
		}
	};
	indices.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
}

pub fn write_obj_bundle(obj_path: &Path, groups: &[MeshGroup]) -> ExportReport {
	let mtl_path = obj_path.with_extension("mtl");
	let mtl_name = mtl_path
		.file_name()
		.map(|f| f.to_string_lossy().to_string())
		.unwrap_or_else(|| "materials.mtl".to_string());

	let mut report = ExportReport::default();
	let mut obj = BufWriter::new(File::create(obj_path).expect("create obj"));
	writeln!(obj, "# Maybraid character mesh export").unwrap();
	writeln!(obj, "mtllib {}", mtl_name).unwrap();

	let mut mtl = BufWriter::new(File::create(&mtl_path).expect("create mtl"));
	writeln!(mtl, "# Maybraid character mesh export").unwrap();

	let mut v_offset = 0u32;
	for group in groups {
		report.groups += 1;
		writeln!(obj, "g {}", group.name).unwrap();
		writeln!(obj, "o {}", group.name).unwrap();
		writeln!(obj, "usemtl {}", group.material_name).unwrap();
		writeln!(mtl, "newmtl {}", group.material_name).unwrap();
		writeln!(
			mtl,
			"Kd {} {} {}",
			group.diffuse[0],
			group.diffuse[1],
			group.diffuse[2]
		)
		.unwrap();
		writeln!(mtl, "d 1.0").unwrap();

		for p in &group.positions {
			writeln!(obj, "v {} {} {}", p.x, p.y, p.z).unwrap();
			report.vertices += 1;
			report.bounds_min = report.bounds_min.min(*p);
			report.bounds_max = report.bounds_max.max(*p);
		}
		if !group.uvs.is_empty() {
			report.has_uvs = true;
			for uv in &group.uvs {
				writeln!(obj, "vt {} {}", uv.x, 1.0 - uv.y).unwrap();
			}
		}
		if !group.normals.is_empty() {
			report.has_normals = true;
			for n in &group.normals {
				writeln!(obj, "vn {} {} {}", n.x, n.y, n.z).unwrap();
			}
		}

		let has_uv = !group.uvs.is_empty();
		let has_n = !group.normals.is_empty();
		for [a, b, c] in &group.faces {
			let fa = *a + 1 + v_offset;
			let fb = *b + 1 + v_offset;
			let fc = *c + 1 + v_offset;
			if has_uv && has_n {
				writeln!(obj, "f {}/{}/{} {}/{}/{} {}/{}/{}", fa, fa, fa, fb, fb, fb, fc, fc, fc)
					.unwrap();
			} else if has_n {
				writeln!(obj, "f {}//{} {}//{} {}//{}", fa, fa, fb, fb, fc, fc).unwrap();
			} else {
				writeln!(obj, "f {} {} {}", fa, fb, fc).unwrap();
			}
			report.faces += 1;
		}
		v_offset += group.positions.len() as u32;
	}

	obj.flush().unwrap();
	mtl.flush().unwrap();

	report.obj_bytes = std::fs::metadata(obj_path).map(|m| m.len()).unwrap_or(0);
	report.mtl_bytes = std::fs::metadata(&mtl_path).map(|m| m.len()).unwrap_or(0);
	report
}

pub fn assembly_block_reason(world: &bevy::prelude::World, root: Entity) -> Option<&'static str> {
	if world.get::<characters::PendingCharacterAssembly>(root).is_some() {
		return Some("pending assembly queue");
	}
	if world.get::<CharacterRoot>(root).is_none() {
		return Some("missing CharacterRoot");
	}
	let Some(members) = world.get::<CharacterMembers>(root) else {
		return Some("missing CharacterMembers");
	};

	for member in members.iter() {
		if world.get::<SocketRefRoot>(member).is_some()
			&& world.get::<SocketRefApplied>(member).is_none()
		{
			return Some("socket ref not fulfilled");
		}
		if world.get::<NeedsSkinRemap>(member).is_some()
			|| world.get::<NeedsDuplicateScenePrune>(member).is_some()
		{
			return Some("skin remap / prune pending");
		}
		if world.get::<CharacterRig>(member).is_some() {
			let bone_map = world.get::<BoneMap>(member);
			if bone_map.is_none_or(|map| map.by_name.is_empty()) {
				return Some("rig bone map empty");
			}
		}
	}

	for mesh_entity in part_mesh_entities(world, root) {
		let Some(mesh) = world.get::<Mesh3d>(mesh_entity) else {
			continue;
		};
		if world.resource::<Assets<Mesh>>().get(&mesh.0).is_none() {
			return Some("mesh asset not loaded");
		}
	}

	if part_mesh_entities(world, root).is_empty() {
		return Some("no part meshes");
	}
	None
}

pub fn assembly_ready(world: &bevy::prelude::World, root: Entity) -> bool {
	assembly_block_reason(world, root).is_none()
}

pub fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../../assets")
}
