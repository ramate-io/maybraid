//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::VegetationOnTerrainMaterialLib;
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};

/// World-model lib: vegetation-on-terrain (bump-out + Chico + Standard).
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		self.vegetation.fulfill(entity, material_ref, commands);
	}
}

/// Installs [`WorldMaterialLib`] as the single [`MaterialRefPlugin`] for Maybraid World.
///
/// Add this before [`chico_vegetation_on_terrain_playground::VegetationOnTerrainPlugin`] so
/// nested domain fulfill plugins skip.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}
