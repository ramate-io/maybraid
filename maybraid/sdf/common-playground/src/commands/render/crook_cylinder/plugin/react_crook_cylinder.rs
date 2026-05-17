//! Reacts to [`super::super::CrookCylinderHelper`].

use bevy::prelude::*;

use crate::commands::render::crook_cylinder::CrookCylinderHelper;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_crook_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &CrookCylinderHelper), Added<CrookCylinderHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::CrookCylinder(helper.inner),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
