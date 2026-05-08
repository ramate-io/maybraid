//! `render` subcommand plugin: registers leaf plugins and render-level systems (announcer).

mod announcer;

use bevy::prelude::*;

use super::noisy_cylinder::plugin::{
	react_render_helper_noisy_cylinder, NoisyCylinderRenderPlugin,
};
use super::tapered_cylinder::plugin::{
	react_render_helper_tapered_cylinder, TaperedCylinderRenderPlugin,
};

pub use announcer::despawn_render_command_announcer;

pub struct RenderCommandsPlugin;

impl Plugin for RenderCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((TaperedCylinderRenderPlugin, NoisyCylinderRenderPlugin)).add_systems(
			Update,
			announcer::despawn_render_command_announcer
				.after(react_render_helper_noisy_cylinder)
				.after(react_render_helper_tapered_cylinder),
		);
	}
}
