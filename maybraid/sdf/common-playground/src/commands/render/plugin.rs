//! `render` subcommand plugin: registers leaf plugins and render-level systems (announcer).

mod announcer;

use bevy::prelude::*;

use super::ball::plugin::{react_render_helper_ball, BallRenderPlugin};
use super::crook_cylinder::plugin::{
	react_render_helper_crook_cylinder, CrookCylinderRenderPlugin,
};
use super::noisy_ball::plugin::{react_render_helper_noisy_ball, NoisyBallRenderPlugin};
use super::noisy_cylinder::plugin::{
	react_render_helper_noisy_cylinder, NoisyCylinderRenderPlugin,
};
use super::noisy_crook_cylinder::plugin::{
	react_render_helper_noisy_crook_cylinder, NoisyCrookCylinderRenderPlugin,
};
use super::tapered_cylinder::plugin::{
	react_render_helper_tapered_cylinder, TaperedCylinderRenderPlugin,
};

pub use announcer::despawn_render_command_announcer;

pub struct RenderCommandsPlugin;

impl Plugin for RenderCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			TaperedCylinderRenderPlugin,
			NoisyCylinderRenderPlugin,
			CrookCylinderRenderPlugin,
			NoisyCrookCylinderRenderPlugin,
			BallRenderPlugin,
			NoisyBallRenderPlugin,
		))
		.add_systems(
			Update,
			announcer::despawn_render_command_announcer
				.after(react_render_helper_noisy_cylinder)
				.after(react_render_helper_tapered_cylinder)
				.after(react_render_helper_crook_cylinder)
				.after(react_render_helper_noisy_crook_cylinder)
				.after(react_render_helper_ball)
				.after(react_render_helper_noisy_ball),
		);
	}
}
