//! Aggregates subcommand plugins and root react ordering.

use bevy::prelude::*;

use crate::commands::render::braid_grass::plugin::BraidGrassRenderPlugin;
use crate::commands::render::plugin::RenderCommandsPlugin;
use crate::commands::render::tropical_tufts::plugin::TropicalTuftsRenderPlugin;

pub struct PlaygroundCommandsPlugin;

impl Plugin for PlaygroundCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((RenderCommandsPlugin, BraidGrassRenderPlugin, TropicalTuftsRenderPlugin));
	}
}
