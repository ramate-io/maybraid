//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use building_shaders::{init_urban_material_caches, UrbanSurfaceMaterialLib};
use characters::material_lib::{init_character_material_caches, CharacterMaterialLib};
use chico::VegetationOnTerrainMaterialLib;
use firearms::{init_muzzle_flame_caches, MuzzleFlameMaterialLib};
use furniture_shaders::{init_furniture_material_caches, FurnitureMaterialLib};
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};

/// World-model lib: furniture kits, character face / clothing, Richmond urban
/// surfaces, the muzzle flame, then vegetation and Standard.
///
/// Furniture recipes (`furniture_wood`, …) must be claimed **before** urban
/// `wood`. Urban recipes (`stucco`, `wood`, …) and `muzzle_flame` must be claimed
/// **before** vegetation's [`StandardMaterial`] fallback.
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub furniture: FurnitureMaterialLib<'w>,
	pub character: CharacterMaterialLib<'w>,
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub muzzle: MuzzleFlameMaterialLib<'w>,
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.furniture.try_fulfill(entity, material_ref, commands)
			|| self.character.try_fulfill(entity, material_ref, commands)
			|| self.urban.try_fulfill(entity, material_ref, commands)
			|| self.muzzle.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Installs [`WorldMaterialLib`] as the single [`MaterialRefPlugin`] for Maybraid World.
///
/// Add this before vegetation [`layer_stack::Present`]
/// so the nested vegetation material plugin skips installing a second
/// [`MaterialRefPlugin`]. [`characters::material_lib::CharacterMaterialRefPlugin`]
/// and [`building_shaders::UrbanMaterialRefPlugin`] also skip;
/// face / urban recipes are claimed here.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_furniture_material_caches(app);
		init_character_material_caches(app);
		init_urban_material_caches(app);
		init_muzzle_flame_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use characters::material_lib::{ClothingShaderMaterialRefCache, FaceShaderMaterialRefCache};

	use building_shaders::UrbanSurfaceMaterialRefCache;
	use firearms::MuzzleFlameMaterialRefCache;
	use furniture_shaders::FurnitureSurfaceMaterialRefCache;

	use crate::material_lib::WorldMaterialRefPlugin;

	#[test]
	fn world_material_plugin_initializes_character_material_caches() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin);
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<FaceShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<UrbanSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<FurnitureSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<MuzzleFlameMaterialRefCache>());
	}
}
