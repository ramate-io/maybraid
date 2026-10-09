//! The world's layer stack satisfies every `finish` requirement headless.

use bevy::prelude::{
	App, AssetApp, AssetPlugin, Mesh, MinimalPlugins, StandardMaterial, Transform, Vec3,
};
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use durham::{Durham, NearStream, StreamRing, TerrainCellLayout};
use lod::gen::Id;
use lod::hcsg::shared::{HcsgRegions, HcsgStorage};
use lod::LodViewer;
use maybraid_game_mode_discover::InDiscovery;
use maybraid_world::WorldLayersPlugin;
use terrain_layer_model::TerrainStreaming;

type NearRing = InDiscovery<StreamRing<NearStream>>;

fn near_regions_sent(app: &App) -> usize {
	app.world()
		.resource::<bevy::ecs::message::Messages<HcsgRegions<NearRing>>>()
		.iter_current_update_messages()
		.filter(|regions| !regions.boxes.is_empty())
		.count()
}

#[test]
fn discovery_seeds_its_session_and_streams_only_when_enabled() -> anyhow::Result<()> {
	let mut app = App::new();
	// Mob scenes run with the player stack, which this app does not install.
	app.set_error_handler(bevy::ecs::error::warn);
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin, ScenePlugin))
		.init_asset::<Mesh>()
		.init_asset::<StandardMaterial>()
		.init_asset::<bevy::world_serialization::WorldAsset>();
	app.add_plugins(WorldLayersPlugin);
	app.finish();
	app.cleanup();
	app.world_mut()
		.spawn((LodViewer, Transform::from_translation(Vec3::new(40.0, 0.0, 40.0))));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.update();

	let storage = app.world().resource::<HcsgStorage>().clone();
	let layout = storage
		.try_entry::<TerrainCellLayout>(Id::Universal)
		.map_err(|_| anyhow::anyhow!("storage busy"))?;
	anyhow::ensure!(layout.is_some(), "entering Discovery seeds Durham's roots");
	anyhow::ensure!(near_regions_sent(&app) == 0, "the gate holds while streaming is off");

	app.insert_resource(TerrainStreaming::<Durham>::new(true));
	app.update();
	anyhow::ensure!(near_regions_sent(&app) == 1, "streaming sends the near ring");
	Ok(())
}
