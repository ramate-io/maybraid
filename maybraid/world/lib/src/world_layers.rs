//! The world's layer stack: terrain, urbanization, vegetation, mobs and
//! furnishing and language, generated on the shared HCSG runtime and presented around the
//! viewer while Discovery streams.

use barking::{BarkingPresentationPlugin, MobNeighborhood, MobScenePresentationPlugin};
use bevy::prelude::*;
use bikanwarda_encounter::BossEncounterPresentationPlugin;
use chico::{
	BumpOutPresentationPlugin, BumpOutRing, CanopyBumpOut, ChicoPresentationPlugin,
	GroveNeighborhood, MediumCanopyBumpOut,
};
use durham::{
	BackgroundStream, Durham, DurhamSurface, DurhamWorldPlugin, FarStream, NearStream,
	StreamPresentationPlugin, StreamRing, Water,
};
use geneva::{GenevaPlugin, LanguageNeighborhood};
use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
use lod::hcsg::{request_hcsg_session_restart, HcsgBoundsPlugin};
use maputo::{FurnitureNeighborhood, MaputoPresentationPlugin};
use maybraid_game_mode_discover::{Discovery, InDiscovery};
use richmond::{
	AuthoredDevelopments, BuiltPresentationPlugin, DevelopmentNeighborhood, PaddedTerrain,
	RichmondConfig,
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
/// Add after [`crate::WorldMaterialRefPlugin`], which installs the unique world
/// material resolver, and after `PlayerCameraPlugin`, since mob presentation
/// adds `PlayerPlugin` when missing.
pub struct WorldLayersPlugin;

impl Plugin for WorldLayersPlugin {
	fn build(&self, app: &mut App) {
		let richmond = RichmondConfig::world_defaults();
		app.add_plugins((GenerationModePlugin::<Discovery>::initial(), DurhamWorldPlugin {
			seed: WORLD_SEED,
		}))
		.insert_resource(crate::poi::PaddedTerrainPoiGround)
		.add_systems(Update, update_durham_terrain_provider)
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
			// Boss encounter presentation on the near stream (HCSG generation and scene spawning)
			BossEncounterPresentationPlugin::<Ring<NearStream>>::default(),
		))
		.add_systems(
			OnEnter(ActiveGenerationMode::of::<Discovery>()),
			request_hcsg_session_restart,
		);
	}
}

/// Update Durham-backed published terrain height provider.
///
/// Runs each frame to keep the provider synchronized with Durham terrain state.
/// The closure captures cloned terrain resources, avoiding any unsafe World access.
fn update_durham_terrain_provider(
	mut commands: Commands,
	hcsg_storage: Res<lod::hcsg::HcsgStorage>,
	layout: Res<durham::TerrainCellLayout>,
) {
	use crate::terrain_query::PublishedTerrainHeight;
	use durham::SharedTerrainStorage;
	
	// Clone the resources (both are cheaply cloneable: HcsgStorage is Arc-wrapped)
	let storage_clone = (*hcsg_storage).clone();
	let layout_clone = (*layout).clone();
	
	// Create closure that captures cloned resources (no unsafe code needed)
	let provider = PublishedTerrainHeight(Box::new(move |xz: Vec2| -> Option<f32> {
		// Query published padded terrain first (same logic as terrain_height_at)
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		match richmond::PaddedTerrain::<Ground>::published_height_at(&storage_clone, xz.x, xz.y) {
			Ok(Some(height)) => return Some(height),
			Ok(None) => {}
			Err(_) => return None,
		}
		// Fall back to Durham's composed surface (raw terrain without pads)
		storage_clone.try_composed_height_at(&layout_clone, xz.x, xz.y).ok().flatten()
	}));
	
	commands.insert_resource(provider);
}
