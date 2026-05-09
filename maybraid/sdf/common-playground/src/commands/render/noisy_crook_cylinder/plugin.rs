//! `render noisy-crook-cylinder` plugin and react systems.

mod react_noisy_crook_cylinder;

use bevy::prelude::*;

use crate::input::capture_command_line_input;

pub use react_noisy_crook_cylinder::react_render_helper_noisy_crook_cylinder;

pub struct NoisyCrookCylinderRenderPlugin;

impl Plugin for NoisyCrookCylinderRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			react_render_helper_noisy_crook_cylinder.after(capture_command_line_input),
		);
	}
}
