//! Idempotent plugin for the Durham terrain model.

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::index::TerrainEntryStore;
use crate::terrain::presentation::TerrainPresenterState;
use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;

/// Registers Avian (if needed) and resources for the terrain model.
pub struct TerrainPlugin;

impl Default for TerrainPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration of [`TerrainPlugin`].
pub fn register_terrain_plugin(app: &mut App) {
	if app.is_plugin_added::<TerrainPlugin>() {
		return;
	}
	app.add_plugins(TerrainPlugin);
}

impl Plugin for TerrainPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		app.init_resource::<TerrainEntryStore>()
			.init_resource::<TerrainCellLayout>()
			.init_resource::<TerrainPresenterState>();
	}
}
