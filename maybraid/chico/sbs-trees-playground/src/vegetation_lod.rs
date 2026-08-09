//! Vegetation LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.
//!
//! Structural hosts ([`ComponentsOnly<MonsterGrass>`]) nest fine-phase
//! [`FoliageNode`] / [`StickNode`] hosts; both layers listen on the same region channels.

use avian3d::prelude::PhysicsPlugins;
use bevy::prelude::*;
use chico_groves::MonsterGrass;
use chico_vegetation_components::{ComponentsOnly, FoliageNode, StickNode};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodRefreshCorePlugin, LodSceneRefreshRegionPlugin, Spotlight,
};
use lod_avian::AvianLodSceneRefreshPlugin;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Full modern refresh stack for structural + fine-phase vegetation hosts.
///
/// 1. Camera → [`Bullseye`] / [`Spotlight`] region messages  
/// 2. [`PatchSceneBounds`](lod::PatchSceneBounds) stamps host Avian volumes from
///    [`LodScene::scene_bounds`](lod::LodScene::scene_bounds)  
/// 3. Avian region index → level messages for structural and child component hosts  
/// 4. Entity refresh (max fold) + chunk sync / cull
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
		.insert_resource(Spotlight { extent: 20.0 })
		.insert_resource(LodChunkFulfillBudget {
			spawn_weights_per_frame: 256,
			cull_weights_per_frame: 64,
		})
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			// Structural grove hosts.
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<MonsterGrass>,
				VegetationBullseye,
				With<Camera>,
			>::default(),
			AvianLodSceneRefreshPlugin::<
				ComponentsOnly<MonsterGrass>,
				VegetationSpotlight,
				With<Camera>,
			>::default(),
			// Fine-phase stick / foliage hosts nested under structural roots.
			AvianLodSceneRefreshPlugin::<FoliageNode, VegetationBullseye, With<Camera>>::default(),
			AvianLodSceneRefreshPlugin::<FoliageNode, VegetationSpotlight, With<Camera>>::default(),
			AvianLodSceneRefreshPlugin::<StickNode, VegetationBullseye, With<Camera>>::default(),
			AvianLodSceneRefreshPlugin::<StickNode, VegetationSpotlight, With<Camera>>::default(),
		));
	}
}
