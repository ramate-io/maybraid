//! Aggregates subcommand plugins and root react ordering. See `commands/README.md`.

use bevy::prelude::*;

use crate::commands::render::plugin::RenderCommandsPlugin;
use crate::commands::settings::plugin::SettingsCommandsPlugin;

pub struct PlaygroundCommandsPlugin;

impl Plugin for PlaygroundCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(RenderCommandsPlugin).add_plugins(SettingsCommandsPlugin);
	}
}
