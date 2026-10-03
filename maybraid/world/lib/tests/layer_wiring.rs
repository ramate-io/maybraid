//! The world's layer stack satisfies every `finish` requirement headless.

use barking::Barking;
use bevy::prelude::{App, AssetPlugin, MinimalPlugins};
use chico::Chico;
use durham::{Durham, DurhamCells};
use furnishing_layer_model::Furnishing;
use furnishing_layer_presentation::FurnishingPresent;
use layer_stack::ModeSubscribers;
use maputo::Maputo;
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::TrainingGround;
use maybraid_world::WorldLayersPlugin;
use mob_layer_model::{MobGenerationCore, Mobs};
use mob_layer_presentation::MobPresent;
use terrain_layer_model::OnTerrain;
use terrain_layer_presentation::TerrainPresentationCore;
use urbanization_layer_model::Urbanization;
use urbanization_layer_presentation::{PaddedCells, UrbanizationHosts};
use vegetation_layer_model::Vegetation;
use vegetation_layer_presentation::VegetationPresent;

type Urbanized = Urbanization<richmond::Richmond<OnTerrain<Durham>>>;
type Forested = Chico<Urbanized>;
type Vegetated = Vegetation<Forested>;
type Inhabited = Mobs<Barking<Vegetated>>;
type Furnished = Furnishing<Maputo<Urbanized>>;

#[test]
fn layered_world_stack_finishes_headless() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.add_plugins(WorldLayersPlugin);
	app.finish();

	let padded = app.world().resource::<ModeSubscribers<(Urbanized, PaddedCells)>>();
	anyhow::ensure!(padded.contains::<Discovery>());
	anyhow::ensure!(padded.contains::<TrainingGround>());

	let hosts = app.world().resource::<ModeSubscribers<(Urbanized, UrbanizationHosts)>>();
	anyhow::ensure!(hosts.contains::<Discovery>());
	anyhow::ensure!(hosts.contains::<TrainingGround>());

	let vegetation = app.world().resource::<ModeSubscribers<(Vegetated, VegetationPresent)>>();
	anyhow::ensure!(vegetation.contains::<Discovery>());
	anyhow::ensure!(vegetation.contains::<TrainingGround>());

	let mobs = app.world().resource::<ModeSubscribers<(Inhabited, MobPresent)>>();
	anyhow::ensure!(mobs.contains::<Discovery>());
	anyhow::ensure!(mobs.contains::<TrainingGround>());

	let furniture = app.world().resource::<ModeSubscribers<(Furnished, FurnishingPresent)>>();
	anyhow::ensure!(furniture.contains::<Discovery>());
	anyhow::ensure!(furniture.contains::<TrainingGround>());
	anyhow::ensure!(
		app.is_plugin_added::<MobGenerationCore<Barking<Vegetated>>>(),
		"both mob generation plugins share one core"
	);

	anyhow::ensure!(
		app.world()
			.get_resource::<ModeSubscribers<(OnTerrain<Durham>, DurhamCells)>>()
			.is_none(),
		"the world registers no DurhamCells presenter"
	);
	anyhow::ensure!(
		!app.is_plugin_added::<TerrainPresentationCore<OnTerrain<Durham>, DurhamCells>>(),
		"raw Durham present is not the world core"
	);
	anyhow::ensure!(
		app.is_plugin_added::<TerrainPresentationCore<Urbanized, PaddedCells>>(),
		"one terrain presenter core"
	);
	Ok(())
}
