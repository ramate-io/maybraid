//! `render tapered-cylinder` plugin and react systems.

mod react_tapered_cylinder;

use bevy::prelude::*;

use crate::input::capture_command_line_input;

pub use react_tapered_cylinder::react_render_helper_tapered_cylinder;

pub struct TaperedCylinderRenderPlugin;

impl Plugin for TaperedCylinderRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			react_render_helper_tapered_cylinder.after(capture_command_line_input),
		);
	}
}
