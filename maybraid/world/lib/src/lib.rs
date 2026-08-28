//! Assembled world model: Durham terrain, streamed forest, sky dome.
//!
//! Character, water exclusion, and further layers stay out until extents stay
//! playable.

pub mod commands;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{PlaygroundConfig, VegetationOnTerrainPlugin};
use game_commands::command::GameCommandPlugin;
use game_commands::ui::GameCommandDrawerConfig;
use maybraid_sky::SkyDomePlugin;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(VegetationOnTerrainPlugin {
			config: PlaygroundConfig::world_defaults(),
			commands: false,
		})
		.add_plugins(SkyDomePlugin::default())
		.add_plugins(
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
				.with_drawer_config(GameCommandDrawerConfig {
					open_at_start: false,
					toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
					..default()
				}),
		)
		.add_systems(
			Update,
			ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
		);
	}
}
