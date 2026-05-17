//! Reacts to [`super::super::NoisyBallHelper`].

use bevy::prelude::*;

use sdf_common::NoisySurface;

use crate::commands::render::noisy_ball::NoisyBallHelper;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_noisy_ball(
	mut commands: Commands,
	q: Query<(Entity, &NoisyBallHelper), Added<NoisyBallHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		let ball = helper.inner.ball;
		let noise = helper.inner.resolved_noise();
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::NoisyBall(NoisySurface::from_params(ball, noise)),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
