//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::VegetationOnTerrainMaterialLib;
use crozon_characters::material_lib::{init_crozon_material_caches, ClothingMaterialLib};
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};

/// World-model lib: character clothing/firearms, then vegetation and Standard.
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub character: ClothingMaterialLib<'w>,
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.character.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Installs [`WorldMaterialLib`] as the single [`MaterialRefPlugin`] for Maybraid World.
///
/// Add this before [`chico_vegetation_on_terrain_playground::VegetationOnTerrainPlugin`] so
/// nested domain fulfill plugins skip.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_crozon_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use crozon_characters::material_lib::ClothingShaderMaterialRefCache;

	use crate::material_lib::WorldMaterialRefPlugin;

	#[test]
	fn world_material_plugin_initializes_character_material_cache() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin);
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
	}
}
