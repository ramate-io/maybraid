//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::VegetationOnTerrainMaterialLib;
use crozon_characters::material_lib::{init_crozon_material_caches, CrozonMaterialLib};
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};
use richmond_building_shaders::{init_richmond_urban_material_caches, UrbanSurfaceMaterialLib};

/// World-model lib: Crozon face / clothing, Richmond urban surfaces, then vegetation and Standard.
///
/// Urban recipes (`stucco`, `wood`, …) must be claimed **before** vegetation's
/// [`StandardMaterial`] fallback or streamed kits never run the urban PBR shader.
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub crozon: CrozonMaterialLib<'w>,
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
		self.crozon.try_fulfill(entity, material_ref, commands)
			|| self.urban.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Installs [`WorldMaterialLib`] as the single [`MaterialRefPlugin`] for Maybraid World.
///
/// Add this before [`chico_vegetation_on_terrain_playground::VegetationOnTerrainPlugin`] so
/// nested domain fulfill plugins skip. [`crozon_characters::material_lib::CrozonMaterialRefPlugin`]
/// and [`richmond_building_shaders::RichmondUrbanMaterialRefPlugin`] also skip; face / urban
/// recipes are claimed here.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_crozon_material_caches(app);
		init_richmond_urban_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use crozon_characters::material_lib::{
		ClothingShaderMaterialRefCache, FaceShaderMaterialRefCache,
	};

	use richmond_building_shaders::UrbanSurfaceMaterialRefCache;

	use crate::material_lib::WorldMaterialRefPlugin;

	#[test]
	fn world_material_plugin_initializes_character_material_caches() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin);
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<FaceShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<UrbanSurfaceMaterialRefCache>());
	}
}
