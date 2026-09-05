//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	init_vegetation_on_terrain_material_caches, VegetationOnTerrainMaterialLib,
};
use crozon_characters::material_lib::{init_crozon_material_caches, ClothingMaterialLib};
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};
use richmond_building_shaders::{init_richmond_urban_material_caches, UrbanSurfaceMaterialLib};

/// Clothing, then urban surfaces, then vegetation (Standard last).
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub clothing: ClothingMaterialLib<'w>,
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.clothing.try_fulfill(entity, material_ref, commands)
			|| self.urban.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Single world [`MaterialRefPlugin`]. Add before vegetation / developments.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_crozon_material_caches(app);
		init_richmond_urban_material_caches(app);
		init_vegetation_on_terrain_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}
