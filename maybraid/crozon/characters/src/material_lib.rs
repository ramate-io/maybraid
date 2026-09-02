//! Crozon [`MaterialLib`]: clothing shader recipes, then [`StandardMaterial`] fallthrough.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crozon_character_items::ClothingMaterial;
use crozon_character_shaders::ClothingShaderMaterial;
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Inserts clothing material caches. Idempotent.
pub fn init_crozon_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ClothingShaderMaterialRefCache>();
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
			MaterialId::Name(name) if ClothingMaterial::is_clothing_recipe(name) => {
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

/// Multi-type lib: clothing recipes, then green [`StandardMaterial`] default.
#[derive(SystemParam)]
pub struct CrozonMaterialLib<'w> {
	pub clothing: ClothingMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for CrozonMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.clothing.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers clothing caches + [`MaterialRefPlugin`] for [`CrozonMaterialLib`].
pub struct CrozonMaterialRefPlugin;

impl Plugin for CrozonMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_crozon_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<CrozonMaterialLib<'_>>::default());
	}
}
