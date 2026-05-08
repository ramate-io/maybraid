//! `render noisy-cylinder` plugin and react systems.

mod react_noisy_cylinder;

use bevy::prelude::*;

use crate::input::capture_command_line_input;

pub use react_noisy_cylinder::react_render_helper_noisy_cylinder;

pub struct NoisyCylinderRenderPlugin;

impl Plugin for NoisyCylinderRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			react_render_helper_noisy_cylinder.after(capture_command_line_input),
		);
	}
}
