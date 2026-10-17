//! Wavefront OBJ writer for assembled character meshes.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use bevy::asset::Assets;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::relationship::RelationshipTarget;
use bevy::math::{Affine3A, Vec2, Vec3};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::{GlobalTransform, Mesh, Mesh3d, Name};
use characters::{
	assembly::CharacterPartSlot,
	member::CharacterMembers,
	nodes::PartNode,
	rig::{BoneMap, CharacterPart, CharacterRig, NeedsDuplicateScenePrune, NeedsSkinRemap},
	CharacterRoot,
};
use material_ref::MaterialRefRoot;
use rigs::{SocketRefApplied, SocketRefRoot};

use crate::obj_text::validate_obj_text;
use crate::skin::skin_mesh_world;
use crate::uv::{islands_from_uvs, islands_overlap, pack_part_uvs};

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
	pub validation_ok: bool,
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
			validation_ok: false,
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
		entry
			.faces
			.extend(faces.into_iter().map(|[a, b, c]| [a + base, b + base, c + base]));
	}

	let mut groups: Vec<MeshGroup> = groups.into_values().collect();
	groups.sort_by(|a, b| a.name.cmp(&b.name));
	groups
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
	mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(|a| a.len()).unwrap_or(0)
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
		let positions: Vec<Vec3> =
			skinned.positions.iter().map(|p| root_inverse.transform_point3(*p)).collect();
		let normals: Vec<Vec3> = skinned
			.normals
			.iter()
			.map(|n| root_inverse.transform_vector3(*n).normalize())
			.collect();
		(positions, normals)
	} else {
		let world_tf = world.get::<GlobalTransform>(entity).expect("global");
		let local = root_inverse * world_tf.affine();
		let positions: Vec<Vec3> =
			mesh_positions(mesh).iter().map(|p| local.transform_point3(*p)).collect();
		let normals: Vec<Vec3> = mesh_normals(mesh)
			.iter()
			.map(|n| local.transform_vector3(*n).normalize())
			.collect();
		(positions, normals)
	};

	let mut faces = faces;
	align_winding(&positions, &normals, &mut faces);
	let uvs = if uvs.len() == positions.len() { uvs } else { Vec::new() };
	let normals = if normals.len() == positions.len() { normals } else { Vec::<Vec3>::new() };
	(positions, normals, uvs, faces)
}

/// Flip triangle winding so geometric normal agrees with vertex normals (Blender culls CW).
fn align_winding(positions: &[Vec3], normals: &[Vec3], faces: &mut [[u32; 3]]) {
	if normals.len() != positions.len() {
		return;
	}
	for face in faces.iter_mut() {
		let [ia, ib, ic] = *face;
		let (Some(&pa), Some(&pb), Some(&pc)) =
			(positions.get(ia as usize), positions.get(ib as usize), positions.get(ic as usize))
		else {
			continue;
		};
		let (Some(&na), Some(&nb), Some(&nc)) =
			(normals.get(ia as usize), normals.get(ib as usize), normals.get(ic as usize))
		else {
			continue;
		};
		let geo = (pb - pa).cross(pc - pa);
		if geo.dot(na + nb + nc) < 0.0 {
			*face = [ia, ic, ib];
		}
	}
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
	let n = mesh_positions(mesh).len() as u32;
	let indices: Vec<u32> = match mesh.indices() {
		Some(Indices::U32(i)) => i.clone(),
		Some(Indices::U16(i)) => i.iter().map(|&x| x as u32).collect(),
		None => (0..n).collect(),
	};
	match mesh.primitive_topology() {
		PrimitiveTopology::TriangleList => indices
			.chunks_exact(3)
			.filter_map(|c| in_range_tri(c[0], c[1], c[2], n))
			.collect(),
		PrimitiveTopology::TriangleStrip => {
			let mut faces = Vec::new();
			if indices.len() < 3 {
				return faces;
			}
			for i in 0..indices.len() - 2 {
				let (a, b, c) = if i % 2 == 0 {
					(indices[i], indices[i + 1], indices[i + 2])
				} else {
					(indices[i + 1], indices[i], indices[i + 2])
				};
				if let Some(tri) = in_range_tri(a, b, c, n) {
					faces.push(tri);
				}
			}
			faces
		}
		// Point / line / fan topologies are not triangle meshes — do not invent faces.
		_ => Vec::new(),
	}
}

fn in_range_tri(a: u32, b: u32, c: u32, n: u32) -> Option<[u32; 3]> {
	if a == b || b == c || a == c || a >= n || b >= n || c >= n {
		None
	} else {
		Some([a, b, c])
	}
}

/// Format Wavefront OBJ text. `v` / `vt` / `vn` are independent global streams
/// (OBJ spec). Face corners must not reuse the vertex index for missing UVs.
pub fn format_obj(groups: &[MeshGroup], mtl_name: &str) -> String {
	let mut obj = String::new();
	obj.push_str("# Maybraid character mesh export\n");
	obj.push_str(&format!("mtllib {mtl_name}\n"));

	let mut v_base = 0u32;
	let mut vt_base = 0u32;
	let mut vn_base = 0u32;
	for group in groups {
		obj.push_str(&format!("g {}\n", group.name));
		obj.push_str(&format!("o {}\n", group.name));
		obj.push_str(&format!("usemtl {}\n", group.material_name));

		for p in &group.positions {
			obj.push_str(&format!("v {} {} {}\n", p.x, p.y, p.z));
		}
		let has_uv = !group.uvs.is_empty() && group.uvs.len() == group.positions.len();
		if has_uv {
			for uv in &group.uvs {
				obj.push_str(&format!("vt {} {}\n", uv.x, 1.0 - uv.y));
			}
		}
		let has_n = !group.normals.is_empty() && group.normals.len() == group.positions.len();
		if has_n {
			for n in &group.normals {
				obj.push_str(&format!("vn {} {} {}\n", n.x, n.y, n.z));
			}
		}

		let nverts = group.positions.len() as u32;
		for [a, b, c] in &group.faces {
			if *a >= nverts || *b >= nverts || *c >= nverts {
				continue;
			}
			obj.push_str("f ");
			obj.push_str(&face_corner(
				v_base + *a + 1,
				has_uv,
				vt_base + *a + 1,
				has_n,
				vn_base + *a + 1,
			));
			obj.push(' ');
			obj.push_str(&face_corner(
				v_base + *b + 1,
				has_uv,
				vt_base + *b + 1,
				has_n,
				vn_base + *b + 1,
			));
			obj.push(' ');
			obj.push_str(&face_corner(
				v_base + *c + 1,
				has_uv,
				vt_base + *c + 1,
				has_n,
				vn_base + *c + 1,
			));
			obj.push('\n');
		}
		v_base += group.positions.len() as u32;
		if has_uv {
			vt_base += group.uvs.len() as u32;
		}
		if has_n {
			vn_base += group.normals.len() as u32;
		}
	}
	obj
}

fn face_corner(v: u32, has_uv: bool, vt: u32, has_n: bool, vn: u32) -> String {
	match (has_uv, has_n) {
		(true, true) => format!("{v}/{vt}/{vn}"),
		(false, true) => format!("{v}//{vn}"),
		(true, false) => format!("{v}/{vt}"),
		(false, false) => format!("{v}"),
	}
}

/// Unwrap missing UVs and pack every part into its own 0..1 atlas tile.
pub fn apply_atlas_uvs(groups: &mut [MeshGroup]) {
	let mut parts: Vec<(Vec<Vec3>, Vec<Vec2>)> =
		groups.iter().map(|g| (g.positions.clone(), g.uvs.clone())).collect();
	pack_part_uvs(&mut parts);
	for (group, (_, uvs)) in groups.iter_mut().zip(parts) {
		group.uvs = uvs;
	}
}

pub fn write_obj_bundle(obj_path: &Path, groups: &[MeshGroup]) -> ExportReport {
	let mut groups = groups.to_vec();
	apply_atlas_uvs(&mut groups);

	let mtl_path = obj_path.with_extension("mtl");
	let mtl_name = mtl_path
		.file_name()
		.map(|f| f.to_string_lossy().to_string())
		.unwrap_or_else(|| "materials.mtl".to_string());

	let mut report = ExportReport::default();
	report.groups = groups.len();
	for group in &groups {
		report.vertices += group.positions.len();
		report.faces += group.faces.len();
		report.has_uvs |= !group.uvs.is_empty();
		report.has_normals |= !group.normals.is_empty();
		for p in &group.positions {
			report.bounds_min = report.bounds_min.min(*p);
			report.bounds_max = report.bounds_max.max(*p);
		}
	}

	let obj_text = format_obj(&groups, &mtl_name);
	std::fs::write(obj_path, &obj_text).expect("write obj");

	let mut mtl = BufWriter::new(File::create(&mtl_path).expect("create mtl"));
	writeln!(mtl, "# Maybraid character mesh export").unwrap();
	for group in &groups {
		writeln!(mtl, "newmtl {}", group.material_name).unwrap();
		writeln!(mtl, "Kd {} {} {}", group.diffuse[0], group.diffuse[1], group.diffuse[2]).unwrap();
		writeln!(mtl, "d 1.0").unwrap();
	}
	mtl.flush().unwrap();

	let islands = islands_from_uvs(&groups.iter().map(|g| g.uvs.clone()).collect::<Vec<_>>());
	let atlas_ok = !islands_overlap(&islands) && islands.iter().all(|i| i.in_unit_square());
	if !atlas_ok {
		eprintln!("obj validation: UV atlas islands overlap or leave 0..1");
	}

	report.validation_ok = match validate_obj_text(&obj_text) {
		Ok(_) => atlas_ok,
		Err(errors) => {
			for error in errors {
				eprintln!("obj validation: {error}");
			}
			false
		}
	};

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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::obj_text::{validate_obj_text, ObjValidationError};
	use crate::uv::{islands_from_uvs, islands_overlap, UvIsland};

	fn tri(name: &str, z: f32, uvs: bool) -> MeshGroup {
		MeshGroup {
			name: name.into(),
			material_name: format!("{name}_mat"),
			diffuse: [0.5, 0.5, 0.5],
			positions: vec![Vec3::new(0.0, 0.0, z), Vec3::new(1.0, 0.0, z), Vec3::new(0.0, 1.0, z)],
			normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
			uvs: if uvs { vec![Vec2::ZERO, Vec2::X, Vec2::Y] } else { Vec::new() },
			faces: vec![[0, 1, 2]],
		}
	}

	#[test]
	fn mixed_uv_groups_keep_vt_in_range() {
		// UV-less body then UV head — the old writer reused the vertex index for vt.
		// format_obj itself still emits v//vn when a group has no UVs; atlas packing
		// is applied in write_obj_bundle so shipped files always have vt.
		let text = format_obj(&[tri("body", 0.0, false), tri("head", 1.0, true)], "x.mtl");
		let stats = validate_obj_text(&text).expect(&text);
		assert_eq!(stats.iter().map(|s| s.faces).collect::<Vec<_>>(), vec![1, 1]);
		assert!(text.contains("f 4/1/4 5/2/5 6/3/6"), "{text}");
		assert!(!text.contains("f 4/4/4"), "{text}");
	}

	#[test]
	fn atlas_pack_writes_non_overlapping_vt_islands() {
		let mut groups =
			vec![tri("eye_left", 0.0, true), tri("eye_right", 0.0, true), tri("body", 1.0, false)];
		apply_atlas_uvs(&mut groups);
		let packed: Vec<Vec<Vec2>> = groups.iter().map(|g| g.uvs.clone()).collect();
		let islands = islands_from_uvs(&packed);
		assert_eq!(islands.len(), 3);
		assert!(!islands_overlap(&islands), "{islands:?}");
		for group in &groups {
			assert_eq!(group.uvs.len(), group.positions.len());
		}

		let text = format_obj(&groups, "x.mtl");
		let stats = validate_obj_text(&text).expect(&text);
		assert!(stats.iter().all(|s| s.texcoords == s.vertices && s.faces == 1), "{stats:?}");
		assert!(text.contains("f 1/1/1"), "{text}");
		assert!(text.contains("f 4/4/4"), "{text}");
		assert!(text.contains("f 7/7/7"), "{text}");
	}

	#[test]
	fn shipped_epiphant_obj_islands_do_not_overlap() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../../../../art/characters/exports/epiphant/epiphant_default_preview.obj");
		let text = std::fs::read_to_string(&path).expect("read shipped obj");
		let stats = validate_obj_text(&text).expect(&text);
		assert_eq!(stats.len(), 8, "{stats:?}");
		assert!(stats.iter().all(|s| s.texcoords == s.vertices && s.faces > 0), "{stats:?}");

		let mut current = None::<String>;
		let mut by_obj: HashMap<String, Vec<Vec2>> = HashMap::new();
		for line in text.lines() {
			let line = line.trim();
			if let Some(name) = line.strip_prefix("o ") {
				current = Some(name.trim().to_string());
			} else if let Some(rest) = line.strip_prefix("vt ") {
				let mut parts = rest.split_whitespace();
				let u: f32 = parts.next().and_then(|s| s.parse().ok()).expect("vt u");
				let v: f32 = parts.next().and_then(|s| s.parse().ok()).expect("vt v");
				if let Some(name) = &current {
					by_obj.entry(name.clone()).or_default().push(Vec2::new(u, v));
				}
			}
		}
		assert_eq!(by_obj.len(), 8, "{:?}", by_obj.keys().collect::<Vec<_>>());
		let islands: Vec<UvIsland> =
			by_obj.values().filter_map(|uvs| UvIsland::from_uvs(uvs)).collect();
		assert_eq!(islands.len(), 8);
		assert!(!islands_overlap(&islands), "{islands:?}");
		for island in &islands {
			assert!(island.in_unit_square(), "{island:?}");
		}
	}

	#[test]
	fn zero_faces_or_oob_indices_fail_validation() {
		let empty_faces = MeshGroup {
			name: "tail".into(),
			material_name: "tail_mat".into(),
			diffuse: [1.0, 1.0, 1.0],
			positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
			normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
			uvs: Vec::new(),
			faces: Vec::new(),
		};
		let text = format_obj(&[empty_faces], "x.mtl");
		let err = validate_obj_text(&text).unwrap_err();
		assert!(
			err.iter().any(
				|e| matches!(e, ObjValidationError::ZeroFaces { group, .. } if group == "tail")
			),
			"{err:?}"
		);

		let oob = "o head\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nf 1/9/1 2/9/1 3/9/1\n";
		let err = validate_obj_text(oob).unwrap_err();
		assert!(
			err.iter()
				.any(|e| matches!(e, ObjValidationError::IndexOutOfRange { kind: "vt", .. })),
			"{err:?}"
		);
	}
}
