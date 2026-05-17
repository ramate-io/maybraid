//! Aggregates subcommand plugins and root react ordering.

use bevy::prelude::*;

use crate::commands::render::plugin::RenderCommandsPlugin;

pub struct PlaygroundCommandsPlugin;

impl Plugin for PlaygroundCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(RenderCommandsPlugin);
	}
}
