//! Vegetation LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.
//!
//! Structural hosts ([`ComponentsOnly`]`<Grove>`) nest fine-phase
//! [`FoliageNode`] / [`StickNode`] hosts; both layers listen on the same region channels.
//! Cull uses a rotating [`OpenLattice`] annulus (not a full-host scan).

use avian3d::prelude::PhysicsPlugins;
use bevy::prelude::*;
use chico_groves::{LevantineScrub, MonsterGrass, StrangeOasis, TropicalThicket};
use chico_vegetation_components::{ComponentsOnly, FoliageNode, StickNode};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_avian::{AvianLodSceneCullPlugin, AvianLodSceneRefreshPlugin};

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationCull;

/// Register Avian refresh + cull for one structural [`ComponentsOnly`]`<G>` grove host.
macro_rules! structural_grove_lod {
	($app:expr, $grove:ty) => {{
		$app.add_plugins((
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<$grove>,
				VegetationBullseye,
				With<Camera>,
			>::without_full_scan_cull(),
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<$grove>,
				VegetationSpotlight,
				With<Camera>,
			>::without_full_scan_cull(),
			AvianLodSceneCullPlugin::<ComponentsOnly<$grove>, VegetationCull, With<Camera>>::default(),
		));
	}};
}

/// Full modern refresh stack for structural + fine-phase vegetation hosts.
///
/// 1. Camera → [`Bullseye`] / [`Spotlight`] region messages  
/// 2. [`PatchSceneBounds`](lod::PatchSceneBounds) stamps host Avian volumes from
///    [`LodScene::scene_bounds`](lod::LodScene::scene_bounds)  
/// 3. Avian region index → level messages for structural and child component hosts  
/// 4. Entity refresh (max fold) + chunk sync  
/// 5. [`OpenLattice`] cull regions → Avian index → budgeted root teardown
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
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
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, VegetationCull>::default(),
			// Fine-phase stick / foliage hosts nested under structural roots.
			AvianLodSceneRefreshPlugin::<FoliageNode, VegetationBullseye, With<Camera>>::without_full_scan_cull(),
			AvianLodSceneRefreshPlugin::<FoliageNode, VegetationSpotlight, With<Camera>>::without_full_scan_cull(),
			AvianLodSceneCullPlugin::<FoliageNode, VegetationCull, With<Camera>>::default(),
			AvianLodSceneRefreshPlugin::<StickNode, VegetationBullseye, With<Camera>>::without_full_scan_cull(),
			AvianLodSceneRefreshPlugin::<StickNode, VegetationSpotlight, With<Camera>>::without_full_scan_cull(),
			AvianLodSceneCullPlugin::<StickNode, VegetationCull, With<Camera>>::default(),
		));

		// Structural grove hosts (levels + chunk; cull via lattice).
		structural_grove_lod!(app, MonsterGrass);
		structural_grove_lod!(app, LevantineScrub);
		structural_grove_lod!(app, StrangeOasis);
		structural_grove_lod!(app, TropicalThicket);
	}
}
