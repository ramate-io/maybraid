//! Vegetation LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.
//!
//! Fine-phase domain hosts ([`FoliageNode`], [`StickNode`]) stay registered for
//! leftover nested kit nodes. Isolated `/show` plants use
//! [`FlattenedComponentsOnly`]`<`[`PlacedVegetation`]`<`[`std::sync::Arc`]`<T>>>`.
//! Live forest High/Medium emit kits under [`ChicoGroveHost`]. Cull uses a
//! rotating [`OpenLattice`] annulus.

use crate::host::ChicoGroveHost;
use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;
use chico_groves::{
	Alpine, AridConiferSapling, BraidGrass, BushScrub, ChristmasTaiga, CommonTufts,
	ConiferMassives, ConiferSapling, DateGrove, Dryland, ForlornSavanna, GoettingenFollow,
	HighBush, JerrysChaparral, JungleLowerMassives, JungleMassives, Leeward, LevantineScrub,
	LowBush, MonsterGrass, OasisDatePalm, Orchard, PalmShade, RiparianGeneral, RiparianMix,
	RiverineGreen, RollingOaks, Shamanhome, SpottyBushes, Storytellers, StrangeOasis, TallGrass,
	TemperateLowerMassives, TemperateMassives, TradeWinds, TropicalThicket, TropicalTufts,
	TropicalUndergrowth, UnendingJungle, Vineyard, WanderingAcacia, WildGrass,
};
use chico_sbs_trees::{
	BraidOakTree, DatePalm, FriendsConifer, HighBushShoots, HonuBanyan, JungleGrowth,
	JungleStorybookTree, KamakuraTorch, LiamsConifer, NorthernConifer, PalmBush, PalmCrown,
	PenmarchTorch, RorysHeadTrained, SimplemansHedge, SopesBanyan, StorybookTree, TemperateConifer,
	TuftPatch, VaseTree, WaialeaPalm,
};
use chico_vegetation_components::{
	FlattenedComponentsOnly, FoliageNode, PlacedVegetation, StickNode,
};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_avian::{AvianLodSceneCullPlugin, AvianLodSceneRefreshPlugin};
use lod_lazy_refs::LodLazyRefsPlugin;
use scene_ref::SceneRefAdmitBudget;

use crate::stick_physics::{register_vegetation_stick_colliders, StickPhysicsPlugin};

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationCull;

/// Isolated `/show` plant host (live groves do not nest these).
type FlattenedPlant<T> = FlattenedComponentsOnly<PlacedVegetation<std::sync::Arc<T>>>;

/// Register Avian refresh + cull for one LOD host type (fine-phase or structural).
macro_rules! avian_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
					AvianLodSceneRefreshPlugin::<$ty, VegetationBullseye, With<Camera>>::without_full_scan_cull(),
					AvianLodSceneRefreshPlugin::<$ty, VegetationSpotlight, With<Camera>>::without_full_scan_cull(),
					AvianLodSceneCullPlugin::<$ty, VegetationCull, With<Camera>>::default(),
				));
	}};
}

/// Isolated `/show` plant hosts also get High-IR stick capsules at High/Medium.
macro_rules! flattened_plant_host {
	($app:expr, $ty:ty) => {{
		avian_host!($app, FlattenedPlant<$ty>);
		register_vegetation_stick_colliders::<FlattenedPlant<$ty>>($app);
	}};
}

/// Grove `LodScene` plus playable stick compound on the same host.
macro_rules! woody_grove_host {
	($app:expr, $ty:ty) => {{
		avian_host!($app, $ty);
		register_vegetation_stick_colliders::<$ty>($app);
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
		// Same sentinel as Durham `TerrainPlugin`: `PhysicsPlugins` is a group.
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<StickPhysicsPlugin>() {
			app.add_plugins(StickPhysicsPlugin);
		}

		app.insert_resource(Bullseye {
			inner: 50.0,
			// Edge length: ±1000 m so produce covers the 1 km present ring (Medium
			// at 350–700 m). `500` is ±250 m and already inside High.
			outer: 2000.0,
		})
		.insert_resource(Spotlight { extent: 50.0 })
		.insert_resource(OpenLattice {
			exclude_extent: 1000.0,
			outer_extent: 5000.0,
			tile_size: 500.0,
		})
		.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
		.insert_resource(LodChunkFulfillBudget {
			spawn_weights_per_frame: 1024,
			cull_weights_per_frame: 128,
			cull_root_despawns_per_frame: 2,
			begins_per_frame: 96,
			begin_scan_per_frame: 384,
			begin_weights_per_frame: 1024,
			begin_prefill_weights_per_job: 32,
			completes_per_frame: 1024,
			..Default::default()
		})
		.insert_resource(SceneRefAdmitBudget { per_frame: 256, new_merge_meshes_per_frame: 64 })
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, VegetationCull>::default(),
		));

		// Fine-phase domain hosts nested under grove LodScene roots.
		avian_host!(app, FoliageNode);
		avian_host!(app, StickNode);

		// Forest / world present: one host wrapping ForestGroveTile ([#652](https://github.com/ramate-io/maybraid/issues/652)).
		woody_grove_host!(app, ChicoGroveHost);

		// Tuft grove roots (LodScene).
		avian_host!(app, MonsterGrass);
		avian_host!(app, BraidGrass);
		avian_host!(app, TropicalTufts);
		avian_host!(app, CommonTufts);
		avian_host!(app, TallGrass);
		avian_host!(app, WildGrass);

		// Woody grove roots (LodScene + playable stick compound).
		woody_grove_host!(app, BushScrub);
		woody_grove_host!(app, TropicalUndergrowth);
		woody_grove_host!(app, LevantineScrub);
		woody_grove_host!(app, StrangeOasis);
		woody_grove_host!(app, TropicalThicket);
		woody_grove_host!(app, RollingOaks);
		woody_grove_host!(app, Orchard);
		woody_grove_host!(app, RiparianGeneral);
		woody_grove_host!(app, ForlornSavanna);
		woody_grove_host!(app, GoettingenFollow);
		woody_grove_host!(app, Vineyard);
		woody_grove_host!(app, Dryland);
		woody_grove_host!(app, Leeward);
		woody_grove_host!(app, TemperateLowerMassives);
		woody_grove_host!(app, TemperateMassives);
		woody_grove_host!(app, Storytellers);
		woody_grove_host!(app, WanderingAcacia);
		woody_grove_host!(app, TradeWinds);
		woody_grove_host!(app, HighBush);
		woody_grove_host!(app, SpottyBushes);
		woody_grove_host!(app, RiverineGreen);
		woody_grove_host!(app, LowBush);
		woody_grove_host!(app, JungleMassives);
		woody_grove_host!(app, JungleLowerMassives);
		woody_grove_host!(app, UnendingJungle);
		woody_grove_host!(app, JerrysChaparral);
		woody_grove_host!(app, RiparianMix);
		woody_grove_host!(app, Alpine);
		woody_grove_host!(app, ChristmasTaiga);
		woody_grove_host!(app, ConiferSapling);
		woody_grove_host!(app, AridConiferSapling);
		woody_grove_host!(app, ConiferMassives);
		woody_grove_host!(app, PalmShade);
		woody_grove_host!(app, Shamanhome);
		woody_grove_host!(app, DateGrove);

		// Isolated /show plants.
		flattened_plant_host!(app, StorybookTree);
		flattened_plant_host!(app, VaseTree);
		flattened_plant_host!(app, JungleStorybookTree);
		flattened_plant_host!(app, BraidOakTree);
		flattened_plant_host!(app, RorysHeadTrained);
		flattened_plant_host!(app, PenmarchTorch);
		flattened_plant_host!(app, KamakuraTorch);
		flattened_plant_host!(app, HighBushShoots);
		flattened_plant_host!(app, JungleGrowth);
		flattened_plant_host!(app, SimplemansHedge);
		flattened_plant_host!(app, PalmBush);
		flattened_plant_host!(app, HonuBanyan);
		flattened_plant_host!(app, SopesBanyan);
		flattened_plant_host!(app, DatePalm);
		flattened_plant_host!(app, WaialeaPalm);
		flattened_plant_host!(app, OasisDatePalm);
		flattened_plant_host!(app, TuftPatch);
		flattened_plant_host!(app, FriendsConifer);
		flattened_plant_host!(app, LiamsConifer);
		flattened_plant_host!(app, NorthernConifer);
		flattened_plant_host!(app, TemperateConifer);
		flattened_plant_host!(app, PalmCrown);

		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
	}
}
