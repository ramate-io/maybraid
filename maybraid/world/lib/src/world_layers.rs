//! The world's layer stack: terrain, urbanization, vegetation, mobs and
//! furnishing and language, generated on the shared HCSG runtime and presented around the
//! viewer while Discovery streams.

use barking::{
	BarkingNodes, BarkingPresentationPlugin, MobNeighborhood, MobScenePresentationPlugin,
};
use bevy::prelude::*;
use chico::{
	BumpOutPresentationPlugin, BumpOutRing, CanopyBumpOut, ChicoPresentationPlugin, ChicoRoots,
	GroveNeighborhood, MediumCanopyBumpOut,
};
use durham::{
	BackgroundStream, Durham, DurhamRoots, DurhamWorldPlugin, FarStream, NearStream,
	StreamPresentationPlugin, StreamRing, Water,
};
use geneva::{GenevaPlugin, GenevaRoots, LanguageNeighborhood};
use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
use lod::hcsg::shared::{HcsgBoundsPlugin, HcsgDemand, HcsgStorage};
use maputo::{FurnitureNeighborhood, MaputoNodes, MaputoPresentationPlugin};
use maybraid_game_mode_discover::{Discovery, InDiscovery};
use richmond::{
	AuthoredDevelopments, BuiltPresentationPlugin, DevelopmentNeighborhood, PaddedTerrain,
	RichmondConfig, RichmondRoots,
};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<richmond::Richmond<Ground>>;

/// Stream `R`'s cells while Discovery streams.
type Ring<R> = InDiscovery<StreamRing<R>>;

/// The world seed every layer derives from.
const WORLD_SEED: u32 = 42;

/// Every world layer at world defaults.
///
/// Add after [`crate::WorldMaterialRefPlugin`], which must install the world
/// material library before any layer installs its own, and after
/// `PlayerCameraPlugin`, since mob presentation adds `PlayerPlugin` when missing.
pub struct WorldLayersPlugin;

impl Plugin for WorldLayersPlugin {
	fn build(&self, app: &mut App) {
		let richmond = RichmondConfig::world_defaults();
		app.add_plugins((GenerationModePlugin::<Discovery>::initial(), DurhamWorldPlugin {
			seed: WORLD_SEED,
		}))
		.insert_resource(richmond.development_config())
		.insert_resource(richmond.urbanization_selection())
		.init_resource::<AuthoredDevelopments>()
		.add_plugins((
			HcsgBoundsPlugin::<Ring<NearStream>>::default(),
			HcsgBoundsPlugin::<Ring<FarStream>>::default(),
			HcsgBoundsPlugin::<Ring<BackgroundStream>>::default(),
			HcsgBoundsPlugin::<InDiscovery<DevelopmentNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<GroveNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<BumpOutRing<CanopyBumpOut>>>::default(),
			HcsgBoundsPlugin::<InDiscovery<BumpOutRing<MediumCanopyBumpOut>>>::default(),
			HcsgBoundsPlugin::<InDiscovery<MobNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<FurnitureNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<LanguageNeighborhood>>::default(),
		))
		.add_plugins((
			StreamPresentationPlugin::<Ring<NearStream>, NearStream, PaddedTerrain<Ground>>::default(),
			StreamPresentationPlugin::<Ring<FarStream>, FarStream, PaddedTerrain<Ground>>::default(),
			StreamPresentationPlugin::<
				Ring<BackgroundStream>,
				BackgroundStream,
				PaddedTerrain<Ground>,
			>::default(),
			StreamPresentationPlugin::<Ring<NearStream>, NearStream, Water>::default(),
			StreamPresentationPlugin::<Ring<FarStream>, FarStream, Water>::default(),
			StreamPresentationPlugin::<Ring<BackgroundStream>, BackgroundStream, Water>::default(),
		))
		.add_plugins((
			BuiltPresentationPlugin::<InDiscovery<DevelopmentNeighborhood>, Ground>::default(),
			ChicoPresentationPlugin::<InDiscovery<GroveNeighborhood>, Urban>::default(),
			BumpOutPresentationPlugin::<
				InDiscovery<BumpOutRing<CanopyBumpOut>>,
				CanopyBumpOut,
				Urban,
			>::default(),
			BumpOutPresentationPlugin::<
				InDiscovery<BumpOutRing<MediumCanopyBumpOut>>,
				MediumCanopyBumpOut,
				Urban,
			>::default(),
			MaputoPresentationPlugin::<InDiscovery<FurnitureNeighborhood>, Urban>::default(),
			BarkingPresentationPlugin::<InDiscovery<MobNeighborhood>, Urban>::default(),
			MobScenePresentationPlugin,
			GenevaPlugin::<InDiscovery<LanguageNeighborhood>, Urban>::default(),
		))
		.add_systems(OnEnter(ActiveGenerationMode::of::<Discovery>()), start_session);
	}
}

/// Ends every subscription and reseeds each layer's roots, so the world
/// regenerates from the current seeds.
fn start_session(
	storage: Res<HcsgStorage>,
	demand: Res<HcsgDemand>,
	durham: DurhamRoots,
	richmond: RichmondRoots,
	chico: ChicoRoots,
	geneva: GenevaRoots,
) {
	demand.advance_epoch();
	durham.reset(&storage);
	richmond.reset::<Ground>(&storage);
	chico.reset::<Urban>(&storage);
	BarkingNodes::clear::<Urban>(&storage);
	MaputoNodes::clear::<Urban>(&storage);
	geneva.reset::<Urban>(&storage);
}
