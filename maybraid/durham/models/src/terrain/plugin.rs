//! Idempotent plugin for the Durham terrain model.

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::collider::{queue_terrain_trimesh_colliders, TerrainFrictionConfig};
use crate::terrain::index::TerrainEntryStore;
use crate::terrain::jersey::{JerseyControllerLayouts, JerseyStampConfigs};
use crate::terrain::marazion::{
	bootstrap_pre_pocket_high_pass_layout, bootstrap_pre_pocket_low_pass_layout,
	MarazionWatershedConfigs,
};
use crate::terrain::presentation::{
	TerrainBackground, TerrainFar, TerrainNear, TerrainPresenterState, TerrainStreamPresenterState,
};
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
		let marazion = MarazionWatershedConfigs::default();
		let pre_pocket_low = bootstrap_pre_pocket_low_pass_layout(&marazion);
		let pre_pocket_high = bootstrap_pre_pocket_high_pass_layout(&marazion);
		app.init_resource::<TerrainEntryStore>()
			.init_resource::<TerrainCellLayout>()
			.init_resource::<JerseyStampConfigs>()
			.init_resource::<JerseyControllerLayouts>()
			.insert_resource(marazion)
			.insert_resource(pre_pocket_low)
			.insert_resource(pre_pocket_high)
			.init_resource::<TerrainPresenterState>()
			.init_resource::<TerrainStreamPresenterState<TerrainNear>>()
			.init_resource::<TerrainStreamPresenterState<TerrainFar>>()
			.init_resource::<TerrainStreamPresenterState<TerrainBackground>>()
			.init_resource::<TerrainFrictionConfig>()
			.add_systems(Update, queue_terrain_trimesh_colliders);
	}
}
