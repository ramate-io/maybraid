//! Durham terrain models: HCSG generation over [`lod::hcsg::HcsgStorage`], SDF meshing.
//!
//! Each model owns an idempotent plugin (e.g. [`terrain::TerrainResourcesPlugin`]). The
//! crate-root [`DurhamTerrainModelsPlugin`] composes those model plugins.

pub mod shared;
pub mod terrain;
pub mod water;

/// Durham's nodes live in the shared HCSG storage; read them through [`TerrainStorage`].
pub use lod::hcsg::HcsgStorage;
pub use shared::{
	BackgroundStream, DurhamPresentationPlugin, DurhamSurface, DurhamWindow, DurhamWorldPlugin,
	FarStream, NearStream, PlayableStreams, SharedTerrainStorage, StreamPresentationPlugin,
	StreamRing, Streamed, TerrainStream, WaterPresentationPlugin,
};
pub use terrain::render::cascade_chunk_for_cell;
pub use terrain::{
	fine_patch_cell_layout, playable_world_cell_layout, register_terrain_plugin,
	retarget_presentation_assets, stream_banded_draws, stream_banded_level, stream_banded_scene,
	terrain_collider_covers_xz, BaseTerrainNoise, CanyonHighPassControllerLayout,
	CanyonLowPassControllerLayout, CanyonStampCell, CascadeChunk, CellTiling, ComposedTerrain,
	Durham, DurhamRoots,
	GeographicBand, GeographicFamily, GeographicFeature, GeographicFeatureId,
	GeographicFeatureKind, MacroCellLayout, MassifHighPassControllerLayout,
	MassifLowPassControllerLayout, MassifStampCell, OuterCellRing, PlateauControllerLayout,
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout, PlateauStampCell,
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout, PocketWaterStampCell,
	PrePocketHighPassLayout, PrePocketLowPassLayout, PreWatershedTerrain, PresentedTerrainScene,
	RollingHighPassControllerLayout, RollingLowPassControllerLayout, RollingStampCell,
	StreamBandedLod, Terrain, TerrainBackground, TerrainBackgroundRegionPresenter,
	TerrainCellLayout, TerrainCellRing, TerrainColliderEpoch, TerrainColliderMeshSource,
	TerrainColliderSystems, TerrainConfig, TerrainCoverage, TerrainFar, TerrainFarRegionPresenter,
	TerrainFrictionConfig, TerrainHeightSnapshot, TerrainLayoutPinned,
	TerrainMeshBuilder, TerrainMeshLodBand, TerrainNear, TerrainNearRegionPresenter,
	TerrainPresentPending, TerrainPresentationAssets, TerrainPresentationDirty,
	TerrainPresenterState, TerrainRegionPresenter, TerrainRenderItem, TerrainResourcesPlugin,
	TerrainRetarget, TerrainSdf, TerrainStampConfigs, TerrainStorage, TerrainStreamMarker,
	TerrainStreamPresenterState, TerrainStreamRegionPresenter, TerrainSuperseded,
	TerrainTrimeshCollider, TerrainVisualHost, ValleyHighPassControllerLayout,
	ValleyLowPassControllerLayout, ValleyStampCell, WaterSurfaceSnapshot, WatershedBandPass,
	WatershedConfigs, WatershedLeafBounds, WatershedLeafKind, WorldBaseTerrain, MACRO_CELL_SIZE,
	TERRAIN_CELL_SIZE, TERRAIN_FRICTION, WORLD_FINE_HALF_EXTENT_CELLS, WORLD_OUTER_2X_ROWS,
	WORLD_OUTER_4X_ROWS,
};
pub use water::{
	register_water_plugin, ComposedWater, PresentedWaterScene, Water, WaterColumn, WaterPlugin,
	WaterPresentationAssets, WaterPresenterState, WaterRegionPresenter,
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
pub fn register_durham_plugin(app: &mut App) {
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
