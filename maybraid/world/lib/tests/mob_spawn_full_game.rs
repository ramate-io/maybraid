//! Regression test for #1616: MobGroupsPlugin double-add panic.
//!
//! The previous implementation added MobGroupsPlugin in both WorldPlugin::build
//! and install_mob_scenes, causing a panic. This test verifies the fix.

use barking::MobGroupsPlugin;
use bevy::prelude::*;
use maybraid_world::WorldLayersPlugin;

#[test]
fn world_plugin_does_not_double_add_mob_groups() -> anyhow::Result<()> {
	// This test verifies that MobGroupsPlugin can be added explicitly before
	// WorldPlugin without causing a panic. The bug was that both WorldPlugin
	// and WorldLayersPlugin's MobScenePresentationPlugin tried to add it.
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(AssetPlugin::default());
	
	// Add MobGroupsPlugin explicitly (as WorldPlugin::build does)
	app.add_plugins(MobGroupsPlugin);
	
	// Add WorldLayersPlugin which internally calls install_mob_scenes
	// This should NOT panic with "plugin was already added"
	app.add_plugins(WorldLayersPlugin);
	
	app.finish();
	app.cleanup();

	eprintln!("✓ MobGroupsPlugin can be added before WorldLayersPlugin without panic");
	Ok(())
}
