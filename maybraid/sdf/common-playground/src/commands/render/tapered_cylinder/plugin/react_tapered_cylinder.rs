//! Reacts to [`super::super::TaperedCylinderHelper`].

use bevy::prelude::*;

use crate::commands::render::tapered_cylinder::TaperedCylinderHelper;
use crate::input::CommandConsoleOutput;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

pub fn react_render_helper_tapered_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &TaperedCylinderHelper), Added<TaperedCylinderHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::TaperedCylinder(helper.inner),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
