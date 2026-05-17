//! Registers `settings` subcommand react systems.

use bevy::prelude::*;

use crate::commands::render::plugin::despawn_render_command_announcer;

use super::react_checker_size::react_settings_checker_size;
use super::react_seed::react_settings_seed;
use super::react_settings_announcer::despawn_settings_command_announcer;

pub struct SettingsCommandsPlugin;

impl Plugin for SettingsCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(react_settings_checker_size, react_settings_seed, despawn_settings_command_announcer)
				.chain()
				.after(despawn_render_command_announcer),
		);
	}
}
