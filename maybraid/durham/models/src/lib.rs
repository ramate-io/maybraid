//! Durham terrain models: LOD generation, Avian spatial index, SDF meshing.
//!
//! Each model owns an idempotent plugin (e.g. [`terrain::TerrainPlugin`]). The
//! crate-root [`DurhamTerrainModelsPlugin`] composes those model plugins.

pub mod terrain;
pub mod water;

pub use terrain::render::cascade_chunk_for_cell;
pub use terrain::{
	register_terrain_plugin, AvianTerrainIndex, BaseTerrainNoise, CanyonHighPassControllerLayout,
	CanyonLowPassControllerLayout, CanyonStampCell, ComposedTerrain, JerseyControllerLayouts,
	JerseyStampConfigs, MacroCellLayout, MassifHighPassControllerLayout,
	MassifLowPassControllerLayout, MassifStampCell, MarazionWatershedConfigs,
	PlateauControllerLayout, PlateauHighPassControllerLayout, PlateauLowPassControllerLayout,
	PlateauStampCell, PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout,
	PocketWaterStampCell, PrePocketLayout, PreWatershedTerrain, RollingHighPassControllerLayout,
	RollingLowPassControllerLayout, RollingStampCell, Terrain, TerrainCellId, TerrainCellLayout,
	TerrainConfig, TerrainEntryStore, TerrainPlugin, TerrainPresentationAssets,
	TerrainPresenterState, TerrainRegionPresenter, TerrainRenderItem, TerrainSdf, TerrainStoreView,
	TerrainTrimeshCollider, ValleyHighPassControllerLayout, ValleyLowPassControllerLayout,
	ValleyStampCell, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};
pub use water::{
	register_water_plugin, ComposedWater, Water, WaterPlugin, WaterPresentationAssets,
	WaterPresenterState, WaterRegionPresenter, WaterStoreView,
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
		register_water_plugin(app);
	}
}
