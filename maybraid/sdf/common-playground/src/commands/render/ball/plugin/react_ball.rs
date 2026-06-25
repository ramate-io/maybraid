//! Reacts to [`super::super::BallHelper`].

use bevy::prelude::*;

use crate::commands::render::ball::BallHelper;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_ball(
	mut commands: Commands,
	q: Query<(Entity, &BallHelper), Added<BallHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::Ball(helper.inner),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
