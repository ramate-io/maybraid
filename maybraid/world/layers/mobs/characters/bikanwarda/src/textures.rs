//! Liam's painted Epiphant maps on the skinned Bikanwarda body.
//!
//! Liam painted Color / Roughness / Metallic PNGs on the
//! [#1589](https://github.com/ramate-io/maybraid/pull/1589) export OBJ. That OBJ is
//! the live part meshes CPU-skinned once and chart-unwrapped, so it is static.
//! `character-mesh-export --rebind-painted` matches every painted triangle back to
//! the runtime part mesh it came from and writes [`PREVIEW_GLB`]: one mesh per
//! component in that part's own local (bind) space, with the painted UVs and the
//! part GLB's own `JOINTS_0` / `WEIGHTS_0`.
//!
//! Bikanwarda keeps the ordinary Epiphant part nodes (skins, sockets, mirrors).
//! [`BikanwardaCharacterPlugin`] swaps each part's primary mesh for its painted
//! mesh and binds [`StandardMaterial`] by slot name, so the painted surface deforms
//! with the same rig as every other Epiphant. Bevy's metallic-roughness sampler is
//! packed: **G = roughness, B = metallic**, linear. Color is sRGB.

use bevy::camera::primitives::MeshAabb;
use bevy::image::ImageLoaderSettings;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::StandardMaterial;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use character_shaders::FaceShaderMaterial;
use characters::assembly::CharacterPartSlot;
use characters::{
	CharacterPart, NeedsDuplicateScenePrune, NeedsSkinRemap, PartNode, SkinRefApplied, SkinRefRoot,
};
use material_ref::{
	MaterialId, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefRoot, MaterialRefSystems,
};

use crate::presentation::BikanwardaPresentation;

/// Authored preview maps, relative to `maybraid/assets`.
pub const EPIPHANT_DEFAULT_TEXTURE_DIR: &str = "characters/epiphant/default";

/// Painted meshes re-bound to the runtime parts (`#Mesh{i}/Primitive0`, i = [`EpiphantTextureSlot::painted_mesh_index`]).
pub const PREVIEW_GLB: &str = "characters/epiphant/default/epiphant_default_preview.glb";

/// One painted component of the Epiphant default preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EpiphantTextureSlot {
	Body,
	Head,
	EarLeft,
	EarRight,
	EyeLeft,
	EyeRight,
	Nose,
	Tail,
}

impl EpiphantTextureSlot {
	pub const ALL: &'static [Self] = &[
		Self::Body,
		Self::Head,
		Self::EarLeft,
		Self::EarRight,
		Self::EyeLeft,
		Self::EyeRight,
		Self::Nose,
		Self::Tail,
	];

	pub const fn recipe_name(self) -> &'static str {
		match self {
			Self::Body => "body_mat",
			Self::Head => "head_mat",
			Self::EarLeft => "ear_left_mat",
			Self::EarRight => "ear_right_mat",
			Self::EyeLeft => "eye_left_mat",
			Self::EyeRight => "eye_right_mat",
			Self::Nose => "nose_mat",
			Self::Tail => "tail_mat",
		}
	}

	pub const fn part_slot(self) -> CharacterPartSlot {
		match self {
			Self::Body => CharacterPartSlot::BodyMesh,
			Self::Head => CharacterPartSlot::HeadMesh,
			Self::EarLeft => CharacterPartSlot::EarLeft,
			Self::EarRight => CharacterPartSlot::EarRight,
			Self::EyeLeft => CharacterPartSlot::EyeLeft,
			Self::EyeRight => CharacterPartSlot::EyeRight,
			Self::Nose => CharacterPartSlot::Nose,
			Self::Tail => CharacterPartSlot::Tail,
		}
	}

	pub const fn is_eye(self) -> bool {
		matches!(self, Self::EyeLeft | Self::EyeRight)
	}

	/// `o` name in the #1589 OBJ and glTF mesh name.
	pub const fn object_name(self) -> &'static str {
		match self {
			Self::Body => "body",
			Self::Head => "head",
			Self::EarLeft => "ear_left",
			Self::EarRight => "ear_right",
			Self::EyeLeft => "eye_left",
			Self::EyeRight => "eye_right",
			Self::Nose => "nose",
			Self::Tail => "tail",
		}
	}

	/// glTF mesh index in [`PREVIEW_GLB`] (exporter `PAINTED_MESH_ORDER`).
	pub const fn painted_mesh_index(self) -> usize {
		match self {
			Self::Body => 0,
			Self::Head => 1,
			Self::EarLeft => 2,
			Self::EarRight => 3,
			Self::EyeLeft => 4,
			Self::EyeRight => 5,
			Self::Nose => 6,
			Self::Tail => 7,
		}
	}

	/// Asset path of this component's painted mesh.
	pub const fn painted_mesh_path(self) -> &'static str {
		match self {
			Self::Body => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh0/Primitive0"
			}
			Self::Head => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh1/Primitive0"
			}
			Self::EarLeft => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh2/Primitive0"
			}
			Self::EarRight => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh3/Primitive0"
			}
			Self::EyeLeft => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh4/Primitive0"
			}
			Self::EyeRight => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh5/Primitive0"
			}
			Self::Nose => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh6/Primitive0"
			}
			Self::Tail => {
				"characters/epiphant/default/epiphant_default_preview.glb#Mesh7/Primitive0"
			}
		}
	}

	pub const fn color_path(self) -> &'static str {
		match self {
			Self::Body => "characters/epiphant/default/body_mat-Color.png",
			Self::Head => "characters/epiphant/default/head_mat-Color.png",
			Self::EarLeft => "characters/epiphant/default/ear_left_mat-Color.png",
			Self::EarRight => "characters/epiphant/default/ear_right_mat-Color.png",
			Self::EyeLeft => "characters/epiphant/default/eye_left_mat-Color.png",
			Self::EyeRight => "characters/epiphant/default/eye_right_mat-Color.png",
			Self::Nose => "characters/epiphant/default/nose_mat-Color.png",
			Self::Tail => "characters/epiphant/default/tail_mat-Color.png",
		}
	}

	/// Packed linear metallic-roughness (G = roughness, B = metallic).
	pub const fn metallic_roughness_path(self) -> &'static str {
		match self {
			Self::Body => "characters/epiphant/default/body_mat-MetallicRoughness.png",
			Self::Head => "characters/epiphant/default/head_mat-MetallicRoughness.png",
			Self::EarLeft => "characters/epiphant/default/ear_left_mat-MetallicRoughness.png",
			Self::EarRight => "characters/epiphant/default/ear_right_mat-MetallicRoughness.png",
			Self::EyeLeft => "characters/epiphant/default/eye_left_mat-MetallicRoughness.png",
			Self::EyeRight => "characters/epiphant/default/eye_right_mat-MetallicRoughness.png",
			Self::Nose => "characters/epiphant/default/nose_mat-MetallicRoughness.png",
			Self::Tail => "characters/epiphant/default/tail_mat-MetallicRoughness.png",
		}
	}

	pub fn from_part(slot: CharacterPartSlot) -> Option<Self> {
		Self::ALL.iter().copied().find(|item| item.part_slot() == slot)
	}

	pub fn from_recipe_name(name: &str) -> Option<Self> {
		Self::ALL.iter().copied().find(|item| item.recipe_name() == name)
	}

	pub fn from_material_id(id: &MaterialId) -> Option<Self> {
		match id {
			MaterialId::Name(name) => Self::from_recipe_name(name),
			MaterialId::Default => None,
		}
	}
}

/// Recipe stamp for a Bikanwarda part. Eyes keep the form emissive tint in palette[0].
pub fn material_ref_for_slot(
	slot: CharacterPartSlot,
	presentation: &BikanwardaPresentation,
) -> Option<MaterialRef> {
	let texture_slot = EpiphantTextureSlot::from_part(slot)?;
	let material = MaterialRef::named(texture_slot.recipe_name());
	if texture_slot.is_eye() {
		Some(material.with_palette([presentation.eye_tint()]))
	} else {
		Some(material)
	}
}

/// Cache of textured [`StandardMaterial`] handles (not the untextured Name fallback).
pub type EpiphantTextureRefCache = MaterialRefCache<StandardMaterial>;

/// Loads Color (sRGB) + packed metallic-roughness (linear) by slot name and swaps
/// each painted part onto its re-bound painted mesh.
pub struct BikanwardaCharacterPlugin;

impl Plugin for BikanwardaCharacterPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EpiphantTextureRefCache>();
		app.init_resource::<PaintedMeshHandles>();
		app.add_systems(
			Update,
			(apply_epiphant_slot_textures, bind_painted_meshes).after(MaterialRefSystems::Fulfill),
		);
	}
}

fn apply_epiphant_slot_textures(
	mut commands: Commands,
	asset_server: Option<Res<AssetServer>>,
	materials: Option<ResMut<Assets<StandardMaterial>>>,
	mut cache: ResMut<EpiphantTextureRefCache>,
	roots: Query<(Entity, &MaterialRefRoot)>,
	children: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
) {
	let Some(asset_server) = asset_server else {
		return;
	};
	let Some(mut materials) = materials else {
		return;
	};
	for (entity, root) in &roots {
		let Some(slot) = EpiphantTextureSlot::from_material_id(&root.0.name) else {
			continue;
		};
		let handle = textured_handle(&asset_server, &mut materials, &mut cache, slot, &root.0);
		stamp_textured_material(entity, handle, &mut commands, &children, &meshes);
	}
}

fn textured_handle(
	asset_server: &AssetServer,
	materials: &mut Assets<StandardMaterial>,
	cache: &mut EpiphantTextureRefCache,
	slot: EpiphantTextureSlot,
	material_ref: &MaterialRef,
) -> Handle<StandardMaterial> {
	let key = MaterialRefKey::from(material_ref);
	if let Some(handle) = cache.get(&key) {
		return handle;
	}
	let handle = materials.add(standard_material_for_slot(asset_server, slot, material_ref));
	cache.insert(key, handle.clone());
	handle
}

fn load_image(asset_server: &AssetServer, path: &'static str, srgb: bool) -> Handle<Image> {
	asset_server
		.load_builder()
		.with_settings(move |settings: &mut ImageLoaderSettings| {
			settings.is_srgb = srgb;
		})
		.load(path)
}

fn standard_material_for_slot(
	asset_server: &AssetServer,
	slot: EpiphantTextureSlot,
	material_ref: &MaterialRef,
) -> StandardMaterial {
	let emissive = if slot.is_eye() {
		material_ref.palette.first().copied().unwrap_or(Color::BLACK).into()
	} else {
		LinearRgba::BLACK
	};
	StandardMaterial {
		base_color: Color::WHITE,
		base_color_texture: Some(load_image(asset_server, slot.color_path(), true)),
		metallic_roughness_texture: Some(load_image(
			asset_server,
			slot.metallic_roughness_path(),
			false,
		)),
		// Maps fully control the sample: G = roughness, B = metallic.
		metallic: 1.0,
		perceptual_roughness: 1.0,
		emissive,
		cull_mode: None,
		..default()
	}
}

fn stamp_textured_material(
	entity: Entity,
	handle: Handle<StandardMaterial>,
	commands: &mut Commands,
	children: &Query<&Children>,
	meshes: &Query<(), With<Mesh3d>>,
) {
	if meshes.contains(entity) {
		commands
			.entity(entity)
			.try_remove::<MeshMaterial3d<FaceShaderMaterial>>()
			.try_insert(MeshMaterial3d(handle.clone()));
	}
	let Ok(kids) = children.get(entity) else {
		return;
	};
	for child in kids.iter() {
		stamp_textured_material(child, handle.clone(), commands, children, meshes);
	}
}

/// Marks a part mesh entity that now draws its painted mesh (`bound`) or that was
/// left on the original mesh because the painted mesh did not fit it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintedEpiphantMesh {
	pub slot: EpiphantTextureSlot,
	pub bound: bool,
}

/// Painted mesh handle per slot (loaded on first use).
#[derive(Resource, Default)]
pub struct PaintedMeshHandles(pub HashMap<EpiphantTextureSlot, Handle<Mesh>>);

impl PaintedMeshHandles {
	fn get_or_load(
		&mut self,
		slot: EpiphantTextureSlot,
		asset_server: Option<&AssetServer>,
	) -> Option<Handle<Mesh>> {
		if let Some(handle) = self.0.get(&slot) {
			return Some(handle.clone());
		}
		let handle = asset_server?.load(slot.painted_mesh_path());
		self.0.insert(slot, handle.clone());
		Some(handle)
	}
}

type PartReady = (
	With<PartNode>,
	Without<NeedsSkinRemap>,
	Without<NeedsDuplicateScenePrune>,
	Or<(Without<SkinRefRoot>, With<SkinRefApplied>)>,
);

/// Swap each painted part's primary mesh for its re-bound painted mesh.
///
/// Runs once the part's scene is spawned and its skin is remapped to the rig. The
/// [`SkinnedMesh`] (joints + inverse bind poses) stays on the entity, so the painted
/// mesh deforms exactly like the part it replaces.
fn bind_painted_meshes(
	mut commands: Commands,
	asset_server: Option<Res<AssetServer>>,
	meshes: Option<Res<Assets<Mesh>>>,
	mut handles: ResMut<PaintedMeshHandles>,
	parts: Query<(Entity, &CharacterPart, &MaterialRefRoot), PartReady>,
	children: Query<&Children>,
	part_meshes: Query<(&Mesh3d, Option<&SkinnedMesh>, Has<PaintedEpiphantMesh>)>,
) {
	let Some(meshes) = meshes else {
		return;
	};
	for (part, character_part, material) in &parts {
		let Some(slot) = EpiphantTextureSlot::from_material_id(&material.0.name) else {
			continue;
		};
		if slot.part_slot() != character_part.slot {
			continue;
		}
		let Some(painted_handle) = handles.get_or_load(slot, asset_server.as_deref()) else {
			continue;
		};
		let Some(painted) = meshes.get(&painted_handle) else {
			continue;
		};
		let Some(target) = primary_part_mesh(part, &children, &part_meshes, &meshes) else {
			continue;
		};
		let Ok((source_handle, skin, _)) = part_meshes.get(target) else {
			continue;
		};
		let Some(source) = meshes.get(&source_handle.0) else {
			continue;
		};
		match painted_mesh_fits(painted, source, skin) {
			Ok(()) => {
				debug!(
					"Bikanwarda {}: painted mesh bound (skinned={})",
					slot.object_name(),
					skin.is_some()
				);
				commands.entity(target).try_insert((
					Mesh3d(painted_handle),
					PaintedEpiphantMesh { slot, bound: true },
				));
			}
			Err(reason) => {
				warn!("Bikanwarda {}: painted mesh not bound ({reason})", slot.object_name());
				commands.entity(target).try_insert(PaintedEpiphantMesh { slot, bound: false });
			}
		}
	}
}

/// Largest mesh under `part` (skinned first), matching the exporter's choice.
/// `None` while any mesh is still loading or once the part is already handled.
fn primary_part_mesh(
	part: Entity,
	children: &Query<&Children>,
	part_meshes: &Query<(&Mesh3d, Option<&SkinnedMesh>, Has<PaintedEpiphantMesh>)>,
	meshes: &Assets<Mesh>,
) -> Option<Entity> {
	let mut stack = vec![part];
	let mut skinned: Option<(Entity, usize)> = None;
	let mut rigid: Option<(Entity, usize)> = None;
	while let Some(entity) = stack.pop() {
		if let Ok((mesh3d, skin, handled)) = part_meshes.get(entity) {
			if handled {
				return None;
			}
			let count = meshes.get(&mesh3d.0)?.count_vertices();
			let best = if skin.is_some() { &mut skinned } else { &mut rigid };
			if count >= 3 && best.is_none_or(|(_, c)| count > c) {
				*best = Some((entity, count));
			}
		}
		if let Ok(kids) = children.get(entity) {
			stack.extend(kids.iter());
		}
	}
	skinned.or(rigid).map(|(entity, _)| entity)
}

/// The painted mesh must live in the same local space as the mesh it replaces
/// (same bounds) and carry joints exactly when the entity is skinned.
pub fn painted_mesh_fits(
	painted: &Mesh,
	source: &Mesh,
	skin: Option<&SkinnedMesh>,
) -> Result<(), String> {
	let joints = painted.attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
	match (skin, joints) {
		(Some(skin), Some(VertexAttributeValues::Uint16x4(joints))) => {
			let max = joints.iter().flatten().copied().max().unwrap_or(0) as usize;
			if max >= skin.joints.len() {
				return Err(format!("joint {max} out of {} skin joints", skin.joints.len()));
			}
			if painted.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none() {
				return Err("joints without weights".into());
			}
		}
		(Some(_), _) => return Err("part is skinned but painted mesh has no JOINTS_0".into()),
		(None, Some(_)) => return Err("painted mesh is skinned but part is rigid".into()),
		(None, None) => {}
	}
	let (Some(a), Some(b)) = (painted.get_aabb(), source.get_aabb()) else {
		return Err("missing positions".into());
	};
	let extent = Vec3::from(b.half_extents).max_element().max(1e-3);
	let tolerance = extent * 2e-3;
	let center = Vec3::from(a.center).distance(Vec3::from(b.center));
	let size = (Vec3::from(a.half_extents) - Vec3::from(b.half_extents)).abs().max_element();
	if center > tolerance || size > tolerance {
		return Err(format!(
			"bounds differ (center {center:.4}, extent {size:.4}, tolerance {tolerance:.4})"
		));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, PrimitiveTopology};
	use characters::{CharacterComponents, CharacterRecipe};
	use lod::scene::LodSceneLevel;
	use material_ref::MaterialRef;
	use std::collections::BTreeMap;
	use std::path::{Path, PathBuf};

	fn assets_root() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../../assets")
	}

	fn painted_obj() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../../../../art/characters/exports/epiphant/epiphant_default_preview.obj")
	}

	#[test]
	fn every_slot_has_unique_recipe_name_and_part() {
		let mut names = Vec::new();
		let mut parts = Vec::new();
		for slot in EpiphantTextureSlot::ALL {
			assert_eq!(EpiphantTextureSlot::from_recipe_name(slot.recipe_name()), Some(*slot));
			assert_eq!(EpiphantTextureSlot::from_part(slot.part_slot()), Some(*slot));
			assert!(slot.color_path().contains(slot.recipe_name()));
			assert!(slot.color_path().ends_with("-Color.png"));
			assert!(slot.metallic_roughness_path().contains(slot.recipe_name()));
			assert!(slot.metallic_roughness_path().ends_with("-MetallicRoughness.png"));
			assert!(!names.contains(&slot.recipe_name()));
			assert!(!parts.contains(&slot.part_slot()));
			assert_eq!(
				slot.painted_mesh_path(),
				format!("{PREVIEW_GLB}#Mesh{}/Primitive0", slot.painted_mesh_index())
			);
			names.push(slot.recipe_name());
			parts.push(slot.part_slot());
		}
		assert_eq!(names.len(), 8);
		assert!(EpiphantTextureSlot::from_recipe_name("tuft").is_none());
	}

	#[test]
	fn authored_maps_are_on_disk_for_each_slot() {
		let root = assets_root();
		for slot in EpiphantTextureSlot::ALL {
			assert_exists(&root, slot.color_path());
			assert_exists(&root, slot.metallic_roughness_path());
		}
	}

	fn assert_exists(root: &Path, rel: &str) {
		let path = root.join(rel);
		assert!(path.is_file(), "missing texture {}", path.display());
	}

	#[test]
	fn bikanwarda_parts_bind_epiphant_slot_textures() {
		let actor = BikanwardaActorRecipe::default().components();
		let parts = actor.part_nodes_for_level(LodSceneLevel::High).flatten();
		for slot in EpiphantTextureSlot::ALL {
			let part =
				parts.iter().find(|part| part.slot == slot.part_slot()).unwrap_or_else(|| {
					panic!("{} part missing from Bikanwarda recipe", slot.recipe_name())
				});
			assert_eq!(
				part.material.name,
				MaterialId::named(slot.recipe_name()),
				"{:?} must bind recipe {}",
				slot.part_slot(),
				slot.recipe_name()
			);
		}
	}

	/// Minimal GLB reader for the committed asset (JSON + BIN chunk).
	struct Glb {
		json: serde_json::Value,
		bin: Vec<u8>,
	}

	impl Glb {
		fn read(path: &Path) -> Self {
			let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
			assert_eq!(&bytes[0..4], b"glTF");
			let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
			let json = serde_json::from_slice(&bytes[20..20 + json_len]).expect("GLB JSON");
			let bin_start = 20 + json_len;
			let bin = if bytes.len() > bin_start + 8 {
				let bin_len =
					u32::from_le_bytes(bytes[bin_start..bin_start + 4].try_into().unwrap())
						as usize;
				bytes[bin_start + 8..bin_start + 8 + bin_len].to_vec()
			} else {
				Vec::new()
			};
			Self { json, bin }
		}

		fn accessor(&self, index: u64) -> &serde_json::Value {
			&self.json["accessors"][index as usize]
		}

		fn count(&self, index: u64) -> usize {
			self.accessor(index)["count"].as_u64().unwrap() as usize
		}

		/// Tightly packed u16x4 accessor values.
		fn u16x4(&self, index: u64) -> Vec<[u16; 4]> {
			let accessor = self.accessor(index);
			assert_eq!(accessor["componentType"], 5123);
			let view = &self.json["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
			let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
				+ accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
			(0..self.count(index))
				.map(|i| {
					let at = offset + i * 8;
					[0, 1, 2, 3].map(|k| {
						u16::from_le_bytes(self.bin[at + k * 2..at + k * 2 + 2].try_into().unwrap())
					})
				})
				.collect()
		}

		fn vec3_bounds(&self, index: u64) -> (Vec3, Vec3) {
			let accessor = self.accessor(index);
			let read = |key: &str| {
				let v = accessor[key].as_array().expect("POSITION min/max");
				Vec3::new(
					v[0].as_f64().unwrap() as f32,
					v[1].as_f64().unwrap() as f32,
					v[2].as_f64().unwrap() as f32,
				)
			};
			(read("min"), read("max"))
		}
	}

	fn obj_counts(path: &Path) -> BTreeMap<String, (usize, usize)> {
		let text = std::fs::read_to_string(path).expect("read OBJ");
		let mut current = String::new();
		let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
		for line in text.lines() {
			if let Some(name) = line.strip_prefix("o ") {
				current = name.trim().to_string();
			} else if line.starts_with("v ") {
				counts.entry(current.clone()).or_default().0 += 1;
			} else if line.starts_with("f ") {
				counts.entry(current.clone()).or_default().1 += 1;
			}
		}
		counts
	}

	#[test]
	fn painted_glb_has_one_mesh_per_obj_component_with_painted_topology() {
		let glb = Glb::read(&assets_root().join(PREVIEW_GLB));
		let meshes = glb.json["meshes"].as_array().expect("meshes");
		let materials = glb.json["materials"].as_array().expect("materials");
		assert_eq!(meshes.len(), 8);
		let obj = obj_counts(&painted_obj());
		for slot in EpiphantTextureSlot::ALL {
			let mesh = &meshes[slot.painted_mesh_index()];
			assert_eq!(mesh["name"], slot.object_name());
			let primitive = &mesh["primitives"][0];
			let material = primitive["material"].as_u64().unwrap() as usize;
			assert_eq!(materials[material]["name"], slot.recipe_name());
			let attributes = &primitive["attributes"];
			let positions = glb.count(attributes["POSITION"].as_u64().unwrap());
			let uvs = glb.count(attributes["TEXCOORD_0"].as_u64().expect("TEXCOORD_0"));
			let triangles = glb.count(primitive["indices"].as_u64().unwrap()) / 3;
			let (obj_vertices, obj_faces) = obj[slot.object_name()];
			assert_eq!(triangles, obj_faces, "{} triangles", slot.object_name());
			assert_eq!(positions, obj_vertices, "{} vertices", slot.object_name());
			assert_eq!(uvs, positions);
		}
	}

	/// Source part GLB for `slot` as the Epiphant recipe loads it, and whether the
	/// runtime mirrors it in X.
	fn source_part(slot: EpiphantTextureSlot) -> (PathBuf, bool) {
		let actor = BikanwardaActorRecipe::default().components();
		let parts = actor.epiphant.part_nodes_for_level(LodSceneLevel::High).flatten();
		let part = parts.iter().find(|p| p.slot == slot.part_slot()).expect("epiphant part");
		let path = part.scene.path.split('#').next().unwrap().to_string();
		(assets_root().join(path), part.scene.mirror.is_some())
	}

	/// Largest mesh in a part GLB (the exporter's / runtime's primary mesh).
	fn primary_source_mesh(glb: &Glb) -> (usize, &serde_json::Value) {
		glb.json["meshes"]
			.as_array()
			.expect("meshes")
			.iter()
			.enumerate()
			.max_by_key(|(_, mesh)| {
				glb.count(mesh["primitives"][0]["attributes"]["POSITION"].as_u64().unwrap())
			})
			.expect("source mesh")
	}

	#[test]
	fn painted_meshes_live_in_their_source_part_space_with_its_skin() {
		let painted = Glb::read(&assets_root().join(PREVIEW_GLB));
		for slot in EpiphantTextureSlot::ALL {
			let (path, mirrored) = source_part(*slot);
			let source = Glb::read(&path);
			let (source_index, source_mesh) = primary_source_mesh(&source);
			let source_attrs = &source_mesh["primitives"][0]["attributes"];
			let painted_attrs =
				&painted.json["meshes"][slot.painted_mesh_index()]["primitives"][0]["attributes"];

			let (mut smin, mut smax) =
				source.vec3_bounds(source_attrs["POSITION"].as_u64().unwrap());
			if mirrored {
				(smin.x, smax.x) = (-smax.x, -smin.x);
			}
			let (pmin, pmax) = painted.vec3_bounds(painted_attrs["POSITION"].as_u64().unwrap());
			let tolerance = (smax - smin).max_element() * 2e-3;
			assert!(
				pmin.distance(smin) <= tolerance && pmax.distance(smax) <= tolerance,
				"{} painted bounds {pmin:?}..{pmax:?} vs source {}#Mesh{source_index} {smin:?}..{smax:?}",
				slot.object_name(),
				path.display()
			);

			let skinned_source = source_attrs.get("JOINTS_0").is_some();
			assert_eq!(
				painted_attrs.get("JOINTS_0").is_some(),
				skinned_source,
				"{} JOINTS_0 must follow the source part",
				slot.object_name()
			);
			assert_eq!(painted_attrs.get("WEIGHTS_0").is_some(), skinned_source);
			if skinned_source {
				let source_joints = source.json["skins"]
					.as_array()
					.and_then(|skins| {
						skins.iter().map(|s| s["joints"].as_array().unwrap().len()).max()
					})
					.expect("skinned source GLB has a skin");
				let max_joint = painted
					.u16x4(painted_attrs["JOINTS_0"].as_u64().unwrap())
					.iter()
					.flatten()
					.copied()
					.max()
					.unwrap() as usize;
				assert!(max_joint < source_joints, "{} joint {max_joint}", slot.object_name());
				// Bevy drops JOINTS_0 / WEIGHTS_0 unless the mesh sits on a skinned node.
				let node = painted.json["nodes"]
					.as_array()
					.and_then(|nodes| {
						nodes
							.iter()
							.find(|n| n["mesh"].as_u64() == Some(slot.painted_mesh_index() as u64))
					})
					.unwrap_or_else(|| panic!("{} has no node", slot.object_name()));
				let skin =
					&painted.json["skins"][node["skin"].as_u64().expect("skinned node") as usize];
				assert_eq!(skin["joints"].as_array().unwrap().len(), source_joints);
			}
		}
		for slot in [EpiphantTextureSlot::Body, EpiphantTextureSlot::Head] {
			let attrs =
				&painted.json["meshes"][slot.painted_mesh_index()]["primitives"][0]["attributes"];
			assert!(
				attrs.get("JOINTS_0").is_some(),
				"{} must deform with the rig",
				slot.object_name()
			);
		}
	}

	fn test_mesh(min: Vec3, max: Vec3, skinned: bool) -> Mesh {
		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![min.to_array(), [max.x, min.y, min.z], max.to_array()],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
		if skinned {
			mesh.insert_attribute(
				Mesh::ATTRIBUTE_JOINT_INDEX,
				VertexAttributeValues::Uint16x4(vec![[0, 1, 0, 0]; 3]),
			);
			mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.5_f32, 0.5, 0.0, 0.0]; 3]);
		}
		mesh
	}

	fn bind_app(
		painted: Mesh,
		source: Mesh,
		skinned: bool,
	) -> (App, Entity, Handle<Mesh>, Handle<Mesh>) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), BikanwardaCharacterPlugin))
			.init_asset::<Mesh>()
			.init_asset::<Image>()
			.init_asset::<StandardMaterial>();
		let (painted, source) = {
			let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
			(meshes.add(painted), meshes.add(source))
		};
		app.world_mut()
			.resource_mut::<PaintedMeshHandles>()
			.0
			.insert(EpiphantTextureSlot::Body, painted.clone());
		let part = app
			.world_mut()
			.spawn((
				PartNode::default(),
				CharacterPart { slot: CharacterPartSlot::BodyMesh },
				MaterialRefRoot(MaterialRef::named(EpiphantTextureSlot::Body.recipe_name())),
			))
			.id();
		let joints = vec![part, part];
		let mut child = app.world_mut().spawn((Mesh3d(source.clone()), ChildOf(part)));
		if skinned {
			child.insert(SkinnedMesh { joints, ..default() });
		}
		let child = child.id();
		app.update();
		app.update();
		(app, child, painted, source)
	}

	#[test]
	fn skinned_part_draws_painted_mesh_and_keeps_its_skin() {
		let (lo, hi) = (Vec3::new(-1.0, 0.0, -0.5), Vec3::new(1.0, 2.0, 0.5));
		let (app, child, painted, _) =
			bind_app(test_mesh(lo, hi, true), test_mesh(lo, hi, true), true);
		let world = app.world();
		assert_eq!(world.get::<Mesh3d>(child).map(|m| m.0.id()), Some(painted.id()));
		assert!(world.get::<SkinnedMesh>(child).is_some(), "skin must stay on the entity");
		assert_eq!(
			world.get::<PaintedEpiphantMesh>(child),
			Some(&PaintedEpiphantMesh { slot: EpiphantTextureSlot::Body, bound: true })
		);
	}

	#[test]
	fn painted_mesh_in_another_space_is_not_bound() {
		let (lo, hi) = (Vec3::new(-1.0, 0.0, -0.5), Vec3::new(1.0, 2.0, 0.5));
		let shifted = test_mesh(lo + Vec3::Y, hi + Vec3::Y, true);
		let (app, child, _, source) = bind_app(shifted, test_mesh(lo, hi, true), true);
		let world = app.world();
		assert_eq!(world.get::<Mesh3d>(child).map(|m| m.0.id()), Some(source.id()));
		assert_eq!(world.get::<PaintedEpiphantMesh>(child).map(|m| m.bound), Some(false));
	}

	#[test]
	fn skinned_part_rejects_unskinned_painted_mesh() {
		let (lo, hi) = (Vec3::new(-1.0, 0.0, -0.5), Vec3::new(1.0, 2.0, 0.5));
		let (app, child, _, source) =
			bind_app(test_mesh(lo, hi, false), test_mesh(lo, hi, true), true);
		assert_eq!(app.world().get::<Mesh3d>(child).map(|m| m.0.id()), Some(source.id()));
	}

	#[test]
	fn plugin_loads_color_and_packed_mr_onto_the_named_slot() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), BikanwardaCharacterPlugin))
			.init_asset::<Mesh>()
			.init_asset::<Image>()
			.init_asset::<StandardMaterial>();
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::new(1.0, 1.0, 1.0));
		let entity = app
			.world_mut()
			.spawn((
				MaterialRefRoot(MaterialRef::named(EpiphantTextureSlot::Body.recipe_name())),
				Mesh3d(mesh),
			))
			.id();
		app.update();

		let handle = app
			.world()
			.get::<MeshMaterial3d<StandardMaterial>>(entity)
			.expect("body_mat should fulfill StandardMaterial")
			.0
			.clone();
		let material = app
			.world()
			.resource::<Assets<StandardMaterial>>()
			.get(&handle)
			.expect("textured StandardMaterial");
		let color = material.base_color_texture.as_ref().expect("Color map");
		let mr = material.metallic_roughness_texture.as_ref().expect("packed metallic-roughness");
		let color_path = color.path().expect("Color is a path handle");
		let mr_path = mr.path().expect("metallic-roughness is a path handle");
		assert!(
			color_path.path().ends_with("body_mat-Color.png"),
			"body_mat Color path was {color_path:?}"
		);
		assert!(
			mr_path.path().ends_with("body_mat-MetallicRoughness.png"),
			"body_mat metallic-roughness path was {mr_path:?}"
		);
		assert!((material.metallic - 1.0).abs() < f32::EPSILON);
		assert!((material.perceptual_roughness - 1.0).abs() < f32::EPSILON);
	}
}
