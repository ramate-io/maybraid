//! `render crook-cylinder` plugin and react systems.

mod react_crook_cylinder;

use bevy::prelude::*;

use crate::input::capture_command_line_input;

pub use react_crook_cylinder::react_render_helper_crook_cylinder;

pub struct CrookCylinderRenderPlugin;

impl Plugin for CrookCylinderRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			react_render_helper_crook_cylinder.after(capture_command_line_input),
		);
	}
}
