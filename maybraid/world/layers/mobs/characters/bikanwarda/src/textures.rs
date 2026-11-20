//! Slot-named Epiphant maps for Bikanwarda (`epiphant_default_preview`).
//!
//! Liam’s Color / Roughness / Metallic PNGs are bound by material slot name
//! (`body_mat`, `head_mat`, …) onto [`StandardMaterial`]. Bevy’s metallic-roughness
//! sampler is packed at import: **G = roughness, B = metallic**, linear. Color is
//! sRGB. The current assembly GLBs mostly lack `TEXCOORD_0` and none of them
//! declare these slot names, so the maps wait on a matching export — they still
//! load and stamp by name so they apply automatically when that GLB arrives.

use bevy::image::ImageLoaderSettings;
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;
use character_shaders::FaceShaderMaterial;
use characters::assembly::CharacterPartSlot;
use material_ref::{
	MaterialId, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefRoot, MaterialRefSystems,
};

use crate::presentation::BikanwardaPresentation;

/// Authored preview maps, relative to `maybraid/assets`.
pub const EPIPHANT_DEFAULT_TEXTURE_DIR: &str = "characters/epiphant/default";

/// One painted material slot on the forthcoming matching Epiphant GLB.
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

/// Loads Color (sRGB) + packed metallic-roughness (linear) by slot name.
pub struct BikanwardaCharacterPlugin;

impl Plugin for BikanwardaCharacterPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EpiphantTextureRefCache>();
		app.add_systems(Update, apply_epiphant_slot_textures.after(MaterialRefSystems::Fulfill));
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use characters::{CharacterComponents, CharacterRecipe};
	use lod::scene::LodSceneLevel;
	use std::path::{Path, PathBuf};

	fn assets_root() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../../assets")
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
				"{} must bind recipe {}",
				format!("{:?}", slot.part_slot()),
				slot.recipe_name()
			);
		}
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
