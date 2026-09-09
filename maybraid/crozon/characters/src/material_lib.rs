//! Crozon [`MaterialLib`]: face, then clothing shader recipes.
//!
//! [`CrozonMaterialLib`] is the composable domain lib. Standalone apps
//! ([`CrozonMaterialRefPlugin`]) add [`StandardMaterial`] via
//! [`CrozonStandaloneMaterialLib`]. Maybraid World nests [`CrozonMaterialLib`]
//! and falls through to vegetation.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crozon_character_items::{ClothingMaterial, FirearmMaterial};
use crozon_character_shaders::{ClothingShaderMaterial, FaceShaderKind, FaceShaderMaterial};
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Cache of resolved [`FaceShaderMaterial`] handles.
pub type FaceShaderMaterialRefCache = MaterialRefCache<FaceShaderMaterial>;

/// Inserts clothing and face material caches. Idempotent.
pub fn init_crozon_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ClothingShaderMaterialRefCache>()
		.init_resource::<FaceShaderMaterialRefCache>();
}

/// Claims face recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct FaceMaterialLib<'w> {
	pub face_materials: ResMut<'w, Assets<FaceShaderMaterial>>,
	pub face_cache: ResMut<'w, FaceShaderMaterialRefCache>,
}

impl FaceMaterialLib<'_> {
	fn resolve_face(&mut self, material_ref: &MaterialRef) -> Handle<FaceShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.face_cache.get(&key) {
			return handle;
		}
		let handle = self.face_materials.add(FaceShaderMaterial::from_material_ref(material_ref));
		self.face_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for FaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if FaceShaderKind::is_face_recipe(name) => {
				let handle = self.resolve_face(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Claims clothing recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ClothingMaterialLib<'w> {
	pub clothing_materials: ResMut<'w, Assets<ClothingShaderMaterial>>,
	pub clothing_cache: ResMut<'w, ClothingShaderMaterialRefCache>,
}

impl ClothingMaterialLib<'_> {
	fn resolve_clothing(&mut self, material_ref: &MaterialRef) -> Handle<ClothingShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.clothing_cache.get(&key) {
			return handle;
		}
		let handle = self
			.clothing_materials
			.add(ClothingShaderMaterial::from_material_ref(material_ref));
		self.clothing_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for ClothingMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name)
				if ClothingMaterial::is_clothing_recipe(name)
					|| FirearmMaterial::is_firearm_recipe(name) =>
			{
				let handle = self.resolve_clothing(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Face + clothing / firearm recipes. Does not fall through to [`StandardMaterial`].
///
/// Standalone apps use [`CrozonStandaloneMaterialLib`]. Composed apps (Maybraid
/// World) nest this and supply their own fallback.
#[derive(SystemParam)]
pub struct CrozonMaterialLib<'w> {
	pub face: FaceMaterialLib<'w>,
	pub clothing: ClothingMaterialLib<'w>,
}

impl MaterialLib for CrozonMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.face.try_fulfill(entity, material_ref, commands)
			|| self.clothing.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Menu / playground fulfill: Crozon, then green [`StandardMaterial`].
#[derive(SystemParam)]
pub struct CrozonStandaloneMaterialLib<'w> {
	pub crozon: CrozonMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for CrozonStandaloneMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.crozon.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers clothing / face caches + [`MaterialRefPlugin`] for
/// [`CrozonStandaloneMaterialLib`].
///
/// Composed apps that already installed a fulfill plugin (Maybraid World) skip
/// here; they nest [`CrozonMaterialLib`] instead.
pub struct CrozonMaterialRefPlugin;

impl Plugin for CrozonMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_crozon_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<CrozonStandaloneMaterialLib<'_>>::default());
	}
}
