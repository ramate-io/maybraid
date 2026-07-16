//! Durham terrain models: LOD generation, Avian spatial index, SDF meshing.
//!
//! Each model owns an idempotent plugin (e.g. [`terrain::TerrainPlugin`]). The
//! crate-root [`DurhamTerrainModelsPlugin`] composes those model plugins.

pub mod terrain;

pub use terrain::render::cascade_chunk_for_cell;
pub use terrain::{
	register_terrain_plugin, AvianTerrainIndex, BaseTerrainNoise, CanyonLayer, CellTerrainNoise,
	ComposedTerrain, JerseyFamilySummary, JerseyLayerConfigs, JerseyModulations,
	JerseyStampCellLayout, MacroCellLayout,
	PlateauCapLayer, PocketWaterLayer, RollingGroundLayer, RuggedMassifLayer, Terrain,
	TerrainCellId, TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainPlugin,
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainRenderItem,
	TerrainSdf, TerrainStoreView, TerrainTrimeshCollider, ValleyBasinLayer, JERSEY_STAMP_CELL_SIZE,
	JERSEY_STAMP_GRID_OFFSET, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};

use bevy::prelude::*;

/// Crate-level composition of Durham model plugins.
///
/// Add this once at the app root; it registers each model plugin idempotently.
pub struct DurhamTerrainModelsPlugin;

impl Default for DurhamTerrainModelsPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration of [`DurhamTerrainModelsPlugin`].
pub fn register_durham_terrain_models_plugin(app: &mut App) {
	if app.is_plugin_added::<DurhamTerrainModelsPlugin>() {
		return;
	}
	app.add_plugins(DurhamTerrainModelsPlugin);
}

impl Plugin for DurhamTerrainModelsPlugin {
	fn build(&self, app: &mut App) {
		register_terrain_plugin(app);
		// Future models: register_hydrology_plugin(app); …
	}
}
