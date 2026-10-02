//! The world's layer stack satisfies every `finish` requirement headless.

use bevy::prelude::{App, AssetPlugin, MinimalPlugins};
use durham_terrain_models::{Durham, DurhamCells};
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::TrainingGround;
use maybraid_world::WorldLayersPlugin;
use mob_layer_model::MobGenerationCore;
use mob_layer_presentation::MobPresent;
use terrain_layer_model::{ModeSubscribers, OnTerrain};
use terrain_layer_presentation::TerrainPresentationCore;
use urbanization_layer_model::Urbanization;
use urbanization_layer_presentation::{PaddedCells, UrbanizationHosts};
use vegetation_layer_presentation::VegetationPresent;

type Urbanized = Urbanization<OnTerrain<Durham>>;

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

	let vegetation = app.world().resource::<ModeSubscribers<(Urbanized, VegetationPresent)>>();
	anyhow::ensure!(vegetation.contains::<Discovery>());
	anyhow::ensure!(vegetation.contains::<TrainingGround>());

	let mobs = app.world().resource::<ModeSubscribers<(Urbanized, MobPresent)>>();
	anyhow::ensure!(mobs.contains::<Discovery>());
	anyhow::ensure!(mobs.contains::<TrainingGround>());
	anyhow::ensure!(
		app.is_plugin_added::<MobGenerationCore<Urbanized>>(),
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
