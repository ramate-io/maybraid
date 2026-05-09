//! Aggregates subcommand plugins and root react ordering.

use bevy::prelude::*;

use crate::commands::render::plugin::RenderCommandsPlugin;
use crate::commands::root::react_playground_command_root;
use crate::commands::render::plugin::despawn_render_command_announcer;

pub struct PlaygroundCommandsPlugin;

impl Plugin for PlaygroundCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(RenderCommandsPlugin)
			.add_systems(
				Update,
				react_playground_command_root.after(despawn_render_command_announcer),
			);
	}
}
