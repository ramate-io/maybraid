//! Buildings LOD refresh for the firing-range Les Halles stack.

use bevy::prelude::*;
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_avian::{AvianLodSceneCullPlugin, AvianLodSceneRefreshPlugin};
use player::register_motor_traction_physics;
use richmond_building_components::{
	ComponentsOnly, DoorNode, FloorNode, FurnitureNode, JointNode, LabelNode, PanelNode,
	PartitionNode, RoofNode, StairNode,
};
use richmond_buildings::{ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof};

/// Shared produce domain for bullseye and spotlight building refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsRefresh;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
pub type BuildingsBullseye = BuildingsRefresh;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
pub type BuildingsSpotlight = BuildingsRefresh;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsCull;

macro_rules! avian_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
								AvianLodSceneRefreshPlugin::<$ty, BuildingsRefresh, With<Camera>>::without_full_scan_cull(),
								AvianLodSceneCullPlugin::<$ty, BuildingsCull, With<Camera>>::default(),
							));
	}};
}

/// Full modern refresh stack for structural + fine-phase building hosts.
pub struct FiringRangeBuildingsLodPlugin;

impl Plugin for FiringRangeBuildingsLodPlugin {
	fn build(&self, app: &mut App) {
		register_motor_traction_physics(app);
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
		avian_host!(app, ComponentsOnly<MixedUseLesHallesStorey>);
		avian_host!(app, ComponentsOnly<ConnectingStairwell>);
		avian_host!(app, ComponentsOnly<PitchedRoof>);
	}
}
