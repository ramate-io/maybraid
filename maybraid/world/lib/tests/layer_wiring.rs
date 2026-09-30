//! The world's layer stack satisfies every `finish` requirement headless.

use bevy::prelude::{App, AssetPlugin, MinimalPlugins};
use maybraid_world::WorldLayersPlugin;

#[test]
fn layered_world_stack_finishes_headless() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.add_plugins(WorldLayersPlugin);
	app.finish();
}
