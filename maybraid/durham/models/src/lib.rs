//! Durham terrain models: LOD generation, Avian spatial index, SDF meshing.
//!
//! Each model owns an idempotent plugin (e.g. [`terrain::TerrainResourcesPlugin`]). The
//! crate-root [`DurhamTerrainModelsPlugin`] composes those model plugins.
//! World hosts add [`TerrainPlugin`] for [`Durham`] for shaders, mesh caches, and LOD stream.

pub mod terrain;
pub mod water;

pub use terrain::render::cascade_chunk_for_cell;
pub use terrain::{
	origin_cell_ids_for_layout, register_terrain_plugin, terrain_streaming_enabled,
	AvianTerrainIndex, BaseTerrainNoise, CanyonHighPassControllerLayout,
	CanyonLowPassControllerLayout, CanyonStampCell, ComposedTerrain, Durham,
	JerseyControllerLayouts, JerseyStampConfigs, MacroCellLayout, MarazionBandPass,
	MarazionLeafBounds, MarazionLeafKind, MarazionWatershedConfigs, MassifHighPassControllerLayout,
	MassifLowPassControllerLayout, MassifStampCell, OuterCellRing, PlateauControllerLayout,
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout, PlateauStampCell,
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout, PocketWaterStampCell,
	PrePocketHighPassLayout, PrePocketLowPassLayout, PreWatershedTerrain, PresentedTerrainScene,
	RollingHighPassControllerLayout, RollingLowPassControllerLayout, RollingStampCell, Terrain,
	TerrainBackground, TerrainBackgroundRegionPresenter, TerrainCellId, TerrainCellLayout,
	TerrainCellRing, TerrainColliderHost, TerrainColliderMeshSource, TerrainConfig,
	TerrainCoverage, TerrainEntryStore, TerrainFar, TerrainFarRegionPresenter,
	TerrainFrictionConfig, TerrainGenerationInput, TerrainGenerationResult, TerrainHeightSnapshot,
	TerrainLodCell, TerrainLodIndex, TerrainLodPlugin, TerrainLodPresenter, TerrainMeshBuilder,
	TerrainMeshLodBand, TerrainNear, TerrainNearRegionPresenter, TerrainPlugin,
	TerrainPresentationAssets, TerrainPresentationDirty, TerrainPresenterState,
	TerrainRegionPresenter, TerrainRenderItem, TerrainResourcesPlugin, TerrainSdf,
	TerrainStoreView, TerrainStreamMarker, TerrainStreamPresenterState,
	TerrainStreamRegionPresenter, TerrainStreamingEnabled, TerrainTrimeshCollider,
	ValleyHighPassControllerLayout, ValleyLowPassControllerLayout, ValleyStampCell,
	WorldBaseTerrain, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE, TERRAIN_FRICTION,
	WORLD_FINE_HALF_EXTENT_CELLS, WORLD_TERRAIN_BACKGROUND_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M,
	WORLD_TERRAIN_NEAR_RADIUS_M, WORLD_TERRAIN_PRESENT_STEP_M, WORLD_TERRAIN_STREAM_EDGE_M,
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
