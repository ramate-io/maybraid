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
	BoneMap, CharacterMembers, CharacterPart, CharacterPartSlot, CharacterRig, CharacterRoot,
	NeedsDuplicateScenePrune, NeedsSkinRemap, PartNode, PendingCharacterAssembly, SocketRefApplied,
	SocketRefRoot,
};
use material_ref::MaterialRefRoot;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::obj_text::validate_obj_text;
use crate::skin::SkinnedSurface;
use crate::texture::TextureConfig;
use crate::uv::UnwrappedMesh;
use crate::validate::{MeshAudit, MtlTable};

#[derive(Debug, Clone)]
pub struct MeshGroup {
	pub name: String,
	pub diffuse: [f32; 3],
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub uvs: Vec<Vec2>,
	pub faces: Vec<[u32; 3]>,
}

impl MeshGroup {
	pub fn material_name(&self) -> String {
		format!("{}_mat", self.name)
	}

	pub fn color_file(&self) -> String {
		format!("{}_color.png", self.name)
	}

	pub fn diffuse_rgb(&self) -> [u8; 3] {
		[
			(self.diffuse[0].clamp(0.0, 1.0) * 255.0).round() as u8,
			(self.diffuse[1].clamp(0.0, 1.0) * 255.0).round() as u8,
			(self.diffuse[2].clamp(0.0, 1.0) * 255.0).round() as u8,
		]
	}

	pub fn unwrap_charts(&mut self) {
		let normals = if self.normals.len() == self.positions.len() {
			self.normals.clone()
		} else {
			vec![Vec3::Y; self.positions.len()]
		};
		let unwrapped = UnwrappedMesh::from_indexed(&self.positions, &normals, &self.faces);
		self.positions = unwrapped.positions;
		self.normals = unwrapped.normals;
		self.uvs = unwrapped.uvs;
		self.faces = unwrapped.faces;
	}
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
	pub textures: usize,
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
			textures: 0,
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

		let Some((positions, normals, uvs, faces)) =
			extract_mesh_local(mesh, mesh_entity, world, &global_refs, root_inverse)
		else {
			continue;
		};

		let diffuse = part_diffuse(world, mesh_entity);
		let entry = groups.entry(group_name.clone()).or_insert_with(|| MeshGroup {
			name: group_name,
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
			return sanitize_ident(slot_label(part.slot));
		}
		if let Some(part) = world.get::<CharacterPart>(e) {
			return sanitize_ident(slot_label(part.slot));
		}
		current = world.get::<ChildOf>(e).map(|c| c.parent());
	}
	world
		.get::<Name>(entity)
		.map(|n| sanitize_ident(n.as_str()))
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

fn sanitize_ident(name: &str) -> String {
	let mut out = String::with_capacity(name.len());
	for ch in name.chars() {
		if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
			out.push(ch);
		} else {
			out.push('_');
		}
	}
	if out.is_empty() {
		"mesh".into()
	} else {
		out
	}
}

fn part_diffuse(world: &bevy::prelude::World, entity: Entity) -> [f32; 3] {
	let mut current = Some(entity);
	while let Some(e) = current {
		if let Some(root) = world.get::<MaterialRefRoot>(e) {
			let color = root.0.palette.first().copied().unwrap_or_default();
			let s = color.to_srgba();
			return [s.red, s.green, s.blue];
		}
		current = world.get::<ChildOf>(e).map(|c| c.parent());
	}
	[0.75, 0.75, 0.75]
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

type ExtractedMesh = (Vec<Vec3>, Vec<Vec3>, Vec<Vec2>, Vec<[u32; 3]>);

fn extract_mesh_local(
	mesh: &Mesh,
	entity: Entity,
	world: &bevy::prelude::World,
	globals: &[(Entity, &GlobalTransform)],
	root_inverse: Affine3A,
) -> Option<ExtractedMesh> {
	let uvs = mesh_uvs(mesh);
	let mut faces = mesh_faces(mesh);

	let mesh_world = world
		.get::<GlobalTransform>(entity)
		.map(|t| t.affine())
		.unwrap_or(Affine3A::IDENTITY);

	let (positions, normals) = if let Some(skin) = world.get::<SkinnedMesh>(entity) {
		let bind = world
			.resource::<Assets<SkinnedMeshInverseBindposes>>()
			.get(&skin.inverse_bindposes)?;
		let skinned = SkinnedSurface::bake(mesh, skin, bind, globals, mesh_world)?
			.into_root_space(root_inverse);
		skinned.align_winding(&mut faces);
		(skinned.positions, skinned.normals)
	} else {
		let world_tf = world.get::<GlobalTransform>(entity)?;
		let local = root_inverse * world_tf.affine();
		let positions: Vec<Vec3> =
			mesh_positions(mesh).iter().map(|p| local.transform_point3(*p)).collect();
		let normals: Vec<Vec3> = mesh_normals(mesh)
			.iter()
			.map(|n| local.transform_vector3(*n).normalize_or(Vec3::Y))
			.collect();
		let baked = SkinnedSurface { positions: positions.clone(), normals: normals.clone() };
		baked.align_winding(&mut faces);
		(positions, normals)
	};

	let uvs = if uvs.len() == positions.len() { uvs } else { Vec::new() };
	let normals = if normals.len() == positions.len() { normals } else { Vec::<Vec3>::new() };
	Some((positions, normals, uvs, faces))
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

/// Format a finite f32 without scientific notation (Blender/Model I/O friendly).
pub fn obj_f32(value: f32) -> anyhow::Result<String> {
	if !value.is_finite() {
		return Err(anyhow::anyhow!("non-finite float in OBJ stream"));
	}
	Ok(format!("{value:.6}"))
}

pub struct ObjDocument<'a> {
	groups: &'a [MeshGroup],
	mtl_name: &'a str,
}

impl<'a> ObjDocument<'a> {
	pub fn new(groups: &'a [MeshGroup], mtl_name: &'a str) -> Self {
		Self { groups, mtl_name }
	}

	/// `mtllib`, then per object `o` / vertices / `s off` / `usemtl` / faces.
	pub fn to_string(&self) -> anyhow::Result<String> {
		let mut obj = String::new();
		obj.push_str("# Maybraid character mesh export\n");
		obj.push_str(&format!("mtllib {}\n", self.mtl_name));

		let mut v_base = 0u32;
		let mut vt_base = 0u32;
		let mut vn_base = 0u32;
		for group in self.groups {
			obj.push_str(&format!("o {}\n", group.name));
			for p in &group.positions {
				obj.push_str(&format!("v {} {} {}\n", obj_f32(p.x)?, obj_f32(p.y)?, obj_f32(p.z)?));
			}
			if group.uvs.len() != group.positions.len() {
				return Err(anyhow::anyhow!("group {} missing UVs", group.name));
			}
			for uv in &group.uvs {
				obj.push_str(&format!("vt {} {}\n", obj_f32(uv.x)?, obj_f32(1.0 - uv.y)?));
			}
			if group.normals.len() != group.positions.len() {
				return Err(anyhow::anyhow!("group {} missing normals", group.name));
			}
			for n in &group.normals {
				obj.push_str(&format!(
					"vn {} {} {}\n",
					obj_f32(n.x)?,
					obj_f32(n.y)?,
					obj_f32(n.z)?
				));
			}
			obj.push_str("s off\n");
			obj.push_str(&format!("usemtl {}\n", group.material_name()));

			let nverts = group.positions.len() as u32;
			for [a, b, c] in &group.faces {
				if *a >= nverts || *b >= nverts || *c >= nverts {
					continue;
				}
				obj.push_str(&format!(
					"f {}/{}/{} {}/{}/{} {}/{}/{}\n",
					v_base + *a + 1,
					vt_base + *a + 1,
					vn_base + *a + 1,
					v_base + *b + 1,
					vt_base + *b + 1,
					vn_base + *b + 1,
					v_base + *c + 1,
					vt_base + *c + 1,
					vn_base + *c + 1,
				));
			}
			v_base += group.positions.len() as u32;
			vt_base += group.uvs.len() as u32;
			vn_base += group.normals.len() as u32;
		}
		Ok(obj)
	}
}

pub struct MtlDocument<'a> {
	groups: &'a [MeshGroup],
}

impl<'a> MtlDocument<'a> {
	pub fn new(groups: &'a [MeshGroup]) -> Self {
		Self { groups }
	}
}

impl std::fmt::Display for MtlDocument<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "# Maybraid character mesh export")?;
		for group in self.groups {
			writeln!(f, "newmtl {}", group.material_name())?;
			writeln!(f, "Kd 1.000000 1.000000 1.000000")?;
			writeln!(f, "d 1.000000")?;
			writeln!(f, "map_Kd {}", group.color_file())?;
		}
		Ok(())
	}
}

pub fn apply_chart_uvs(groups: &mut [MeshGroup]) {
	for group in groups {
		group.unwrap_charts();
	}
}

pub fn write_obj_bundle(obj_path: &Path, groups: &[MeshGroup]) -> anyhow::Result<ExportReport> {
	write_obj_bundle_with(obj_path, groups, TextureConfig::default())
}

pub fn write_obj_bundle_with(
	obj_path: &Path,
	groups: &[MeshGroup],
	textures: TextureConfig,
) -> anyhow::Result<ExportReport> {
	let mut groups = groups.to_vec();
	apply_chart_uvs(&mut groups);

	let dir = obj_path.parent().unwrap_or(Path::new("."));
	std::fs::create_dir_all(dir)?;
	let mtl_path = obj_path.with_extension("mtl");
	let mtl_name = mtl_path
		.file_name()
		.map(|f| f.to_string_lossy().into_owned())
		.unwrap_or_else(|| "materials.mtl".to_string());

	let maps = textures.write_for_groups(dir, &groups)?;

	let mut vertices = 0usize;
	let mut faces = 0usize;
	let mut has_uvs = false;
	let mut has_normals = false;
	let mut bounds_min = Vec3::splat(f32::MAX);
	let mut bounds_max = Vec3::splat(f32::MIN);
	for group in &groups {
		vertices += group.positions.len();
		faces += group.faces.len();
		has_uvs |= !group.uvs.is_empty();
		has_normals |= !group.normals.is_empty();
		for p in &group.positions {
			bounds_min = bounds_min.min(*p);
			bounds_max = bounds_max.max(*p);
		}
	}
	let mut report = ExportReport {
		groups: groups.len(),
		vertices,
		faces,
		has_uvs,
		has_normals,
		bounds_min,
		bounds_max,
		textures: maps.len(),
		..Default::default()
	};

	let mesh_audit = MeshAudit::groups(&groups);
	if !mesh_audit.ok() {
		for error in &mesh_audit.errors {
			eprintln!("mesh validation: {error}");
		}
	}
	let unique = MeshAudit::unique_assignments(&groups);
	if !unique.ok() {
		for error in &unique.errors {
			eprintln!("assignment validation: {error}");
		}
	}

	let obj_text = ObjDocument::new(&groups, &mtl_name).to_string()?;
	let obj_audit = MeshAudit::obj_text(&obj_text);
	if !obj_audit.ok() {
		for error in &obj_audit.errors {
			eprintln!("obj validation: {error}");
		}
	}
	std::fs::write(obj_path, &obj_text)?;

	let mtl_text = MtlDocument::new(&groups).to_string();
	std::fs::write(&mtl_path, &mtl_text)?;

	let companion = MeshAudit::companion_files(obj_path);
	if !companion.ok() {
		for error in &companion.errors {
			eprintln!("companion validation: {error}");
		}
	}
	let tex_audit = MeshAudit::mtl_textures(&mtl_text, dir, textures);
	if !tex_audit.ok() {
		for error in &tex_audit.errors {
			eprintln!("mtl validation: {error}");
		}
	}

	report.validation_ok = mesh_audit.ok()
		&& unique.ok()
		&& obj_audit.ok()
		&& companion.ok()
		&& tex_audit.ok()
		&& validate_obj_text(&obj_text).is_ok();
	report.obj_bytes = std::fs::metadata(obj_path).map(|m| m.len()).unwrap_or(0);
	report.mtl_bytes = std::fs::metadata(&mtl_path).map(|m| m.len()).unwrap_or(0);
	Ok(report)
}

pub fn write_import_zip(zip_path: &Path, obj_path: &Path) -> anyhow::Result<()> {
	let dir = obj_path.parent().unwrap_or(Path::new("."));
	let stem = obj_path.file_stem().and_then(|s| s.to_str()).unwrap_or("character");
	let obj_name = obj_path
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_else(|| format!("{stem}.obj"));
	let mtl_path = obj_path.with_extension("mtl");
	let mtl_name = mtl_path
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_else(|| format!("{stem}.mtl"));
	let mtl_text = std::fs::read_to_string(&mtl_path)?;
	let maps = MtlTable::parse(&mtl_text).color_files();

	let zip_file = File::create(zip_path)?;
	let mut zip = ZipWriter::new(BufWriter::new(zip_file));
	let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
	let folder = stem;
	zip.start_file(format!("{folder}/{obj_name}"), options)?;
	zip.write_all(&std::fs::read(obj_path)?)?;
	zip.start_file(format!("{folder}/{mtl_name}"), options)?;
	zip.write_all(mtl_text.as_bytes())?;
	for name in maps {
		zip.start_file(format!("{folder}/{name}"), options)?;
		zip.write_all(&std::fs::read(dir.join(&name))?)?;
	}
	zip.start_file(format!("{folder}/README.md"), options)?;
	zip.write_all(import_instructions(stem).as_bytes())?;
	zip.finish()?;
	Ok(())
}

pub fn import_instructions(stem: &str) -> String {
	format!(
		"# {stem} — per-component textures\n\
		\n\
		Extract this ZIP first. Do not import the ZIP itself.\n\
		You should get a folder containing `{stem}.obj`, `{stem}.mtl`, and one `*_color.png`\n\
		per component (body, head, ears, eyes, nose, tail).\n\
		\n\
		Procreate's Gallery → Import often greys out `.obj` (Files UTI). Official import is\n\
		drag-and-drop of the **folder** that holds OBJ + MTL + PNGs\n\
		([handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).\n\
		\n\
		1. Keep the extracted folder On My iPad (not iCloud).\n\
		2. Split View Files + Procreate. Long-press the *folder*, drag onto the Gallery until a\n\
		   green plus appears, drop. That creates a fresh 3D artwork.\n\
		3. Open the model. Each component should be its own texture set.\n\
		4. Disable Paint through mesh. Paint the head: body and both ears must stay unchanged.\n\
		   Paint the left ear: the right ear must stay unchanged.\n\
		\n\
		Each part has its own `map_Kd` image at {res}×{res}. UVs are a chart unwrap per component.\n",
		res = TextureConfig::default().resolution
	)
}

pub fn assembly_block_reason(world: &bevy::prelude::World, root: Entity) -> Option<&'static str> {
	if world.get::<PendingCharacterAssembly>(root).is_some() {
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
	use crate::uv::UvIsland;

	fn tri(name: &str, z: f32) -> MeshGroup {
		MeshGroup {
			name: name.into(),
			diffuse: [0.2, 0.4, 0.6],
			positions: vec![Vec3::new(0.0, 0.0, z), Vec3::new(1.0, 0.0, z), Vec3::new(0.0, 1.0, z)],
			normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
			uvs: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
			faces: vec![[0, 1, 2]],
		}
	}

	#[test]
	fn writer_uses_independent_vt_streams_and_blender_record_order() -> anyhow::Result<()> {
		let mut groups = vec![tri("body", 0.0), tri("head", 1.0)];
		apply_chart_uvs(&mut groups);
		let text = ObjDocument::new(&groups, "x.mtl").to_string()?;
		if !text.contains("mtllib x.mtl") {
			return Err(anyhow::anyhow!("missing mtllib"));
		}
		if !text.contains("usemtl body_mat") || !text.contains("usemtl head_mat") {
			return Err(anyhow::anyhow!("missing per-object usemtl: {text}"));
		}
		if text.contains("\ng ") {
			return Err(anyhow::anyhow!("g declarations split Procreate into extra layers"));
		}
		let body = text.find("o body").ok_or_else(|| anyhow::anyhow!("missing o body"))?;
		let first_v = text.find("\nv ").ok_or_else(|| anyhow::anyhow!("missing v"))?;
		let usemtl = text.find("usemtl ").ok_or_else(|| anyhow::anyhow!("missing usemtl"))?;
		let first_f = text.find("\nf ").ok_or_else(|| anyhow::anyhow!("missing f"))?;
		if !(body < first_v && first_v < usemtl && usemtl < first_f) {
			return Err(anyhow::anyhow!("expected o → v/vt/vn → usemtl → f, got {text}"));
		}
		if !text.contains("s off") {
			return Err(anyhow::anyhow!("expected s off"));
		}
		let stats = validate_obj_text(&text).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		if stats.len() != 2 || stats.iter().any(|s| s.texcoords != s.vertices) {
			return Err(anyhow::anyhow!("{stats:?}"));
		}
		if stats[0].material.as_deref() != Some("body_mat") {
			return Err(anyhow::anyhow!("body material {:?}", stats[0].material));
		}
		Ok(())
	}

	#[test]
	fn chart_unwrap_writes_unit_square_uvs() -> anyhow::Result<()> {
		let mut groups = vec![tri("eye_left", 0.0), tri("eye_right", 0.0), tri("body", 1.0)];
		apply_chart_uvs(&mut groups);
		for group in &groups {
			let island = UvIsland::from_uvs(&group.uvs).ok_or_else(|| anyhow::anyhow!("uvs"))?;
			if !island.in_unit_square() {
				return Err(anyhow::anyhow!("{island:?}"));
			}
			if MeshAudit::group(group).into_iter().any(|e| {
				matches!(
					e,
					crate::validate::ExportCheck::CollapsedUv { .. }
						| crate::validate::ExportCheck::UvOverlap { .. }
				)
			}) {
				return Err(anyhow::anyhow!("audit failed for {}", group.name));
			}
		}
		Ok(())
	}

	#[test]
	fn shipped_epiphant_obj_is_paintable() -> anyhow::Result<()> {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../../../../art/characters/exports/epiphant/epiphant_default_preview.obj");
		let dir = path.parent().ok_or_else(|| anyhow::anyhow!("parent"))?;
		let text = std::fs::read_to_string(&path)?;
		let stats = validate_obj_text(&text).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		if stats.len() != 8 {
			return Err(anyhow::anyhow!("expected 8 objects, got {stats:?}"));
		}
		if stats
			.iter()
			.any(|s| s.texcoords != s.vertices || s.faces == 0 || s.material.is_none())
		{
			return Err(anyhow::anyhow!("{stats:?}"));
		}
		let names: std::collections::HashSet<_> = stats.iter().map(|s| s.name.as_str()).collect();
		for required in ["body", "head", "ear_left", "ear_right"] {
			if !names.contains(required) {
				return Err(anyhow::anyhow!("missing object {required} in {names:?}"));
			}
		}
		let audit = MeshAudit::obj_text(&text);
		if !audit.ok() {
			return Err(anyhow::anyhow!("{:?}", audit.errors));
		}
		let companion = MeshAudit::companion_files(&path);
		if !companion.ok() {
			return Err(anyhow::anyhow!("{:?}", companion.errors));
		}
		let mtl = std::fs::read_to_string(path.with_extension("mtl"))?;
		let tex = MeshAudit::mtl_textures(&mtl, dir, TextureConfig::default());
		if !tex.ok() {
			return Err(anyhow::anyhow!("{:?}", tex.errors));
		}
		let table = MtlTable::parse(&mtl);
		if table.materials.len() != 8 {
			return Err(anyhow::anyhow!("expected 8 materials, got {}", table.materials.len()));
		}
		let mut images = std::collections::HashSet::new();
		for record in &table.materials {
			let map = record.map_kd.as_deref().ok_or_else(|| anyhow::anyhow!("no map_Kd"))?;
			if !images.insert(map) {
				return Err(anyhow::anyhow!("shared image {map}"));
			}
		}
		Ok(())
	}

	#[test]
	fn write_obj_bundle_emits_unique_maps() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let obj_path = dir.path().join("mesh.obj");
		let mut body = tri("body", 0.0);
		body.diffuse = [1.0, 0.0, 0.0];
		let mut head = tri("head", 1.0);
		head.diffuse = [1.0, 0.0, 0.0];
		let textures = TextureConfig::new(16);
		let report = write_obj_bundle_with(&obj_path, &[body, head], textures)?;
		if !report.validation_ok {
			return Err(anyhow::anyhow!("expected validation_ok"));
		}
		let text = std::fs::read_to_string(&obj_path)?;
		if !text.contains("mtllib mesh.mtl") || !text.contains("usemtl body_mat") {
			return Err(anyhow::anyhow!("{text}"));
		}
		let mtl = std::fs::read_to_string(dir.path().join("mesh.mtl"))?;
		if !mtl.contains("map_Kd body_color.png") || !mtl.contains("map_Kd head_color.png") {
			return Err(anyhow::anyhow!("{mtl}"));
		}
		if mtl.contains("map_Kd body_color.png")
			&& mtl.matches("map_Kd body_color.png").count() != 1
		{
			return Err(anyhow::anyhow!("body image reused"));
		}
		let body_png = crate::texture::SolidPng::load(&dir.path().join("body_color.png"))?;
		let head_png = crate::texture::SolidPng::load(&dir.path().join("head_color.png"))?;
		if body_png.rgb != [255, 0, 0] || head_png.rgb != [255, 0, 0] {
			return Err(anyhow::anyhow!("expected identical base colours on distinct files"));
		}
		if body_png.width != 16 {
			return Err(anyhow::anyhow!("resolution {}", body_png.width));
		}
		if !dir.path().join("body_color.png").exists()
			|| !dir.path().join("head_color.png").exists()
		{
			return Err(anyhow::anyhow!("missing unique PNGs"));
		}
		Ok(())
	}

	#[test]
	fn concatenated_faces_use_global_offsets() -> anyhow::Result<()> {
		let mut groups = vec![tri("body", 0.0), tri("head", 1.0)];
		apply_chart_uvs(&mut groups);
		let text = ObjDocument::new(&groups, "x.mtl").to_string()?;
		if !text.contains("f 4/4/4 5/5/5 6/6/6") {
			return Err(anyhow::anyhow!("head faces must use global v/vt/vn offsets:\n{text}"));
		}
		if text.contains("o head") && text.contains("f 1/1/1") {
			let head = text.split("o head").nth(1).unwrap_or("");
			if head.contains("f 1/1/1") {
				return Err(anyhow::anyhow!("head reused body-local indices"));
			}
		}
		Ok(())
	}

	#[test]
	fn zero_faces_or_oob_indices_fail_validation() -> anyhow::Result<()> {
		let empty_faces = MeshGroup {
			name: "tail".into(),
			diffuse: [1.0, 1.0, 1.0],
			positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
			normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
			uvs: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
			faces: Vec::new(),
		};
		let text = ObjDocument::new(&[empty_faces], "x.mtl").to_string()?;
		let Err(err) = validate_obj_text(&text) else {
			return Err(anyhow::anyhow!("expected zero-face error"));
		};
		if !err
			.iter()
			.any(|e| matches!(e, ObjValidationError::ZeroFaces { group, .. } if group == "tail"))
		{
			return Err(anyhow::anyhow!("{err:?}"));
		}

		let oob = "o head\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nf 1/9/1 2/9/1 3/9/1\n";
		let Err(err) = validate_obj_text(oob) else {
			return Err(anyhow::anyhow!("expected oob"));
		};
		if !err
			.iter()
			.any(|e| matches!(e, ObjValidationError::IndexOutOfRange { kind: "vt", .. }))
		{
			return Err(anyhow::anyhow!("{err:?}"));
		}
		Ok(())
	}
}
