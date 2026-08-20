//! Vegetation LOD refresh: bullseye + spotlight → Avian index → levels → chunk sync.
//!
//! Fine-phase domain hosts ([`FoliageNode`], [`StickNode`]) and structural
//! [`ComponentsOnly`] wrappers that band via [`VegetationComponents::structural_lod`]
//! share the same region channels. Cull uses a rotating [`OpenLattice`] annulus.

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
	BraidOakTree, DatePalm, FriendsConifer, HighBushShoots, HonuBanyan, JungleStorybookTree,
	KamakuraTorch, LiamsConifer, NorthernConifer, PalmBush, PalmCrown, PenmarchTorch,
	RorysHeadTrained, SimplemansHedge, SopesBanyan, StorybookTree, TemperateConifer, TuftPatch,
	VaseTree, WaialeaPalm,
};
use chico_vegetation_components::{
	ComponentsOnly, FlattenedComponentsOnly, FoliageNode, PlacedVegetation, StickNode,
};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_avian::{AvianLodSceneCullPlugin, AvianLodSceneRefreshPlugin};
use scene_ref::SceneRefAdmitBudget;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationCull;

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
			spawn_weights_per_frame: 512,
			cull_weights_per_frame: 128,
			begins_per_frame: 48,
			begin_weights_per_frame: 256,
			begin_prefill_weights_per_job: 8,
			completes_per_frame: 16,
		})
		.insert_resource(SceneRefAdmitBudget { per_frame: 64 })
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, VegetationCull>::default(),
		));

		// Fine-phase domain hosts nested under structural roots.
		avian_host!(app, FoliageNode);
		avian_host!(app, StickNode);

		// Structural grove hosts (Monster Grass still flattens via ComponentsOnly).
		avian_host!(app, ComponentsOnly<MonsterGrass>);
		avian_host!(app, ComponentsOnly<BraidGrass>);
		avian_host!(app, ComponentsOnly<TropicalTufts>);
		avian_host!(app, ComponentsOnly<CommonTufts>);
		avian_host!(app, ComponentsOnly<TallGrass>);
		avian_host!(app, ComponentsOnly<WildGrass>);
		avian_host!(app, BushScrub);
		avian_host!(app, TropicalUndergrowth);
		avian_host!(app, LevantineScrub);
		avian_host!(app, StrangeOasis);
		avian_host!(app, TropicalThicket);
		avian_host!(app, RollingOaks);
		avian_host!(app, Orchard);
		avian_host!(app, RiparianGeneral);
		avian_host!(app, ForlornSavanna);
		avian_host!(app, GoettingenFollow);
		avian_host!(app, Vineyard);
		avian_host!(app, Dryland);
		avian_host!(app, Leeward);
		avian_host!(app, TemperateLowerMassives);
		avian_host!(app, TemperateMassives);
		avian_host!(app, Storytellers);
		avian_host!(app, WanderingAcacia);
		avian_host!(app, TradeWinds);
		avian_host!(app, HighBush);
		avian_host!(app, SpottyBushes);
		avian_host!(app, RiverineGreen);
		avian_host!(app, LowBush);
		avian_host!(app, JungleMassives);
		avian_host!(app, JungleLowerMassives);
		avian_host!(app, UnendingJungle);
		avian_host!(app, JerrysChaparral);
		avian_host!(app, RiparianMix);
		avian_host!(app, Alpine);
		avian_host!(app, ChristmasTaiga);
		avian_host!(app, ConiferSapling);
		avian_host!(app, AridConiferSapling);
		avian_host!(app, ConiferMassives);
		avian_host!(app, PalmShade);
		avian_host!(app, Shamanhome);
		avian_host!(app, DateGrove);

		// Structural single-tree / component hosts used by `/show`.
		avian_host!(app, ComponentsOnly<SopesBanyan>);
		avian_host!(app, ComponentsOnly<PenmarchTorch>);
		avian_host!(app, ComponentsOnly<KamakuraTorch>);
		avian_host!(app, ComponentsOnly<RorysHeadTrained>);
		avian_host!(app, ComponentsOnly<StorybookTree>);
		avian_host!(app, ComponentsOnly<VaseTree>);
		avian_host!(app, ComponentsOnly<FriendsConifer>);
		avian_host!(app, ComponentsOnly<NorthernConifer>);
		avian_host!(app, ComponentsOnly<LiamsConifer>);
		avian_host!(app, ComponentsOnly<TemperateConifer>);
		avian_host!(app, ComponentsOnly<HonuBanyan>);
		avian_host!(app, ComponentsOnly<JungleStorybookTree>);
		avian_host!(app, ComponentsOnly<BraidOakTree>);
		avian_host!(app, ComponentsOnly<SimplemansHedge>);
		avian_host!(app, ComponentsOnly<TuftPatch>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<TuftPatch>>);
		avian_host!(app, ComponentsOnly<PalmCrown>);
		avian_host!(app, ComponentsOnly<DatePalm>);
		avian_host!(app, ComponentsOnly<WaialeaPalm>);
		avian_host!(app, ComponentsOnly<PalmBush>);
		avian_host!(app, ComponentsOnly<HighBushShoots>);

		// Nested grove plant hosts (posed + palette materials).
		avian_host!(app, ComponentsOnly<PlacedVegetation<PalmBush>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<HonuBanyan>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<HighBushShoots>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<OasisDatePalm>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<DatePalm>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<PenmarchTorch>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<StorybookTree>>);
		avian_host!(app, FlattenedComponentsOnly<PlacedVegetation<StorybookTree>>);
		avian_host!(app, FlattenedComponentsOnly<PlacedVegetation<std::sync::Arc<StorybookTree>>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<RorysHeadTrained>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<VaseTree>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<BraidOakTree>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<SimplemansHedge>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<LiamsConifer>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<TemperateConifer>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<KamakuraTorch>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<SopesBanyan>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<WaialeaPalm>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<JungleStorybookTree>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<FriendsConifer>>);
		avian_host!(app, ComponentsOnly<PlacedVegetation<NorthernConifer>>);
	}
}
