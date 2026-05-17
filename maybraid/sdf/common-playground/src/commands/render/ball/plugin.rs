//! `render ball` plugin and react systems.

mod react_ball;

use bevy::prelude::*;

pub use react_ball::react_render_helper_ball;

pub struct BallRenderPlugin;

impl Plugin for BallRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_ball);
	}
}
