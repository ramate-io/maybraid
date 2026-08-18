//! Buildings LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.
//!
//! Fine-phase domain hosts ([`PanelNode`], [`PartitionNode`], …) and structural
//! [`ComponentsOnly`] wrappers that band via [`BuildingComponents::structural_lod`]
//! share the same region channels. Cull uses a rotating [`OpenLattice`] annulus.

use avian3d::prelude::PhysicsPlugins;
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
use richmond_buildings::{
	IApartmentFullStorey, LesHallesLivableFullStorey, LivableApartment, LivableApartments,
};
use richmond_buildings::wizards_tower::WizardsTower;

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

/// Full modern refresh stack for structural + fine-phase building hosts.
pub struct BuildingsLodRefreshPlugin;

impl Plugin for BuildingsLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(PhysicsPlugins::default());
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}

		app.insert_resource(Bullseye {
			inner: 50.0,
			outer: 500.0,
		})
		.insert_resource(Spotlight { extent: 50.0 })
		.insert_resource(OpenLattice {
			exclude_extent: 1000.0,
			outer_extent: 5000.0,
			tile_size: 500.0,
		})
		.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
		.insert_resource(LodChunkFulfillBudget {
			spawn_weights_per_frame: 256,
			cull_weights_per_frame: 128,
			begins_per_frame: 48,
		})
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, BuildingsBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, BuildingsSpotlight>::default(),
			LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, BuildingsCull>::default(),
		));

		// Fine-phase domain hosts.
		avian_host!(app, PanelNode);
		avian_host!(app, PartitionNode);
		avian_host!(app, RoofNode);
		avian_host!(app, FloorNode);
		avian_host!(app, StairNode);
		avian_host!(app, DoorNode);
		avian_host!(app, JointNode);
		avian_host!(app, FurnitureNode);
		avian_host!(app, LabelNode);

		// Structural hosts that band via `structural_lod`.
		avian_host!(app, ComponentsOnly<LesHallesLivableFullStorey>);
		avian_host!(app, ComponentsOnly<LivableApartment>);
		avian_host!(app, ComponentsOnly<LivableApartments>);
		avian_host!(app, ComponentsOnly<IApartmentFullStorey>);

		// Custom composite host.
		avian_host!(app, WizardsTower);
	}
}
