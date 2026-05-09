//! `render noisy-ball` plugin and react systems.

mod react_noisy_ball;

use bevy::prelude::*;

use crate::input::capture_command_line_input;

pub use react_noisy_ball::react_render_helper_noisy_ball;

pub struct NoisyBallRenderPlugin;

impl Plugin for NoisyBallRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_noisy_ball.after(capture_command_line_input));
	}
}
