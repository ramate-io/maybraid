//! Reacts to [`super::super::NoisyCylinderHelper`].

use bevy::prelude::*;

use sdf_common::NoisySurface;

use crate::commands::render::noisy_cylinder::NoisyCylinderHelper;
use crate::input::CommandConsoleOutput;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

pub fn react_render_helper_noisy_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &NoisyCylinderHelper), Added<NoisyCylinderHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		let cyl = helper.inner.cylinder;
		let noise = helper.inner.noise;
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::NoisyCylinder(NoisySurface::from_params(cyl, noise)),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
