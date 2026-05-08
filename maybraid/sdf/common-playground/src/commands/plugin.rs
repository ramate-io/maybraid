//! Aggregates subcommand plugins and root react ordering. See `commands/README.md`.

use bevy::prelude::*;

use crate::commands::render::plugin::RenderCommandsPlugin;
use crate::commands::root::react_playground_command_root;
use crate::commands::settings::plugin::SettingsCommandsPlugin;
use crate::commands::settings::react_settings_announcer::despawn_settings_command_announcer;

pub struct PlaygroundCommandsPlugin;

impl Plugin for PlaygroundCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(RenderCommandsPlugin)
			.add_plugins(SettingsCommandsPlugin)
			.add_systems(
				Update,
				react_playground_command_root.after(despawn_settings_command_announcer),
			);
	}
}
