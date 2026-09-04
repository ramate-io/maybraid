//! Avian LOD refresh for Richmond development hosts.

use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_avian::{AvianLodSceneCullPlugin, AvianLodSceneRefreshPlugin};
use richmond_building_components::{
	ComponentsOnly, DoorNode, FloorNode, FurnitureNode, JointNode, LabelNode, PanelNode,
	PartitionNode, RoofNode, StairNode,
};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{
	ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof, RectangularPitchedRoofComplex,
};
use richmond_developments::{
	CircularTower, GalleryColonnade, GalleryTerrace, ShepherdsHouse, ShepherdsHut, SingleHighrise,
	Skybridge, TrazaloidTower,
};
use std::sync::Arc;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsSpotlight;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsCull;

macro_rules! avian_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
						AvianLodSceneRefreshPlugin::<$ty, BuildingsBullseye, With<Camera>>::without_full_scan_cull(),
						AvianLodSceneRefreshPlugin::<$ty, BuildingsSpotlight, With<Camera>>::without_full_scan_cull(),
						AvianLodSceneCullPlugin::<$ty, BuildingsCull, With<Camera>>::default(),
					));
	}};
}

/// Full refresh stack for structural + fine-phase development hosts.
#[derive(Default)]
pub struct DevelopmentsBuildingsLodPlugin;

/// Idempotent registration of [`DevelopmentsBuildingsLodPlugin`].
pub fn register_developments_buildings_lod_plugin(app: &mut App) {
	if app.is_plugin_added::<DevelopmentsBuildingsLodPlugin>() {
		return;
	}
	app.add_plugins(DevelopmentsBuildingsLodPlugin);
}

impl Plugin for DevelopmentsBuildingsLodPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}

		app.insert_resource(Bullseye { inner: 80.0, outer: 500.0 })
			.insert_resource(Spotlight { extent: 80.0 })
			.insert_resource(OpenLattice {
				exclude_extent: 1000.0,
				outer_extent: 5000.0,
				tile_size: 500.0,
			})
			.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
			.insert_resource(LodChunkFulfillBudget {
				spawn_weights_per_frame: 256,
				cull_weights_per_frame: 128,
				cull_root_despawns_per_frame: 2,
				begins_per_frame: 48,
				begin_scan_per_frame: 192,
				begin_weights_per_frame: 256,
				begin_prefill_weights_per_job: 8,
				completes_per_frame: 128,
				..Default::default()
			})
			.add_plugins((
				LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, BuildingsBullseye>::default(),
				LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, BuildingsSpotlight>::default(
				),
				LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, BuildingsCull>::default(),
			));

		avian_host!(app, PanelNode);
		avian_host!(app, PartitionNode);
		avian_host!(app, RoofNode);
		avian_host!(app, FloorNode);
		avian_host!(app, StairNode);
		avian_host!(app, DoorNode);
		avian_host!(app, JointNode);
		avian_host!(app, FurnitureNode);
		avian_host!(app, LabelNode);
		avian_host!(app, ComponentsOnly<Arc<MixedUseLesHallesStorey>>);
		avian_host!(app, ComponentsOnly<ConnectingStairwell>);
		avian_host!(app, ComponentsOnly<PitchedRoof>);
		avian_host!(app, ComponentsOnly<Arc<ShepherdsHouse>>);
		avian_host!(app, ComponentsOnly<Arc<ShepherdsHut>>);
		avian_host!(app, ComponentsOnly<Arc<CircularTower>>);
		avian_host!(app, ComponentsOnly<Arc<TrazaloidTower>>);
		avian_host!(app, ComponentsOnly<GalleryTerrace>);
		avian_host!(app, ComponentsOnly<GalleryColonnade>);
		avian_host!(app, ComponentsOnly<RectangularPitchedRoofComplex>);
		avian_host!(app, ComponentsOnly<Arc<SingleHighrise>>);
		avian_host!(app, WizardsTower);
		avian_host!(app, ComponentsOnly<Arc<Skybridge>>);
	}
}
