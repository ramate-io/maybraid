//! The world's layer stack satisfies every `finish` requirement headless.

use barking::Barking;
use bevy::prelude::{App, AssetPlugin, MinimalPlugins};
use chico::Chico;
use durham::Durham;
use furnishing_layer_model::Furnishing;
use layer_stack::{LayerGenerationCore, ModeSubscribers};
use maputo::Maputo;
use maybraid_game_mode_discover::Discovery;
use maybraid_world::WorldLayersPlugin;
use mob_layer_model::Mobs;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<richmond::Richmond<Ground>>;
type Veg = Vegetation<Chico<Urban>>;
type Mob = Mobs<Barking<Veg>>;
type Furniture = Furnishing<Maputo<Urban>>;

#[test]
fn layered_world_stack_finishes_headless() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.add_plugins(WorldLayersPlugin);
	app.finish();

	let ground = app.world().resource::<ModeSubscribers<Ground>>();
	anyhow::ensure!(ground.contains::<Discovery>());

	let urban = app.world().resource::<ModeSubscribers<Urban>>();
	anyhow::ensure!(urban.contains::<Discovery>());

	let vegetation = app.world().resource::<ModeSubscribers<Veg>>();
	anyhow::ensure!(vegetation.contains::<Discovery>());

	let mobs = app.world().resource::<ModeSubscribers<Mob>>();
	anyhow::ensure!(mobs.contains::<Discovery>());

	let furniture = app.world().resource::<ModeSubscribers<Furniture>>();
	anyhow::ensure!(furniture.contains::<Discovery>());
	anyhow::ensure!(
		app.is_plugin_added::<LayerGenerationCore<Mob>>(),
		"mob generation installs its core"
	);

	Ok(())
}
