//! Reacts to [`super::super::NoisyCrookCylinderHelper`].

use bevy::prelude::*;

use sdf_common::NoisySurface;

use crate::commands::render::noisy_crook_cylinder::NoisyCrookCylinderHelper;
use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;
use game_commands::command::CommandConsoleOutput;

pub fn react_render_helper_noisy_crook_cylinder(
	mut commands: Commands,
	q: Query<(Entity, &NoisyCrookCylinderHelper), Added<NoisyCrookCylinderHelper>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, helper) in &q {
		let crook = helper.inner.crook;
		let noise = helper.inner.resolved_noise();
		*preview = PreviewConfig {
			primitive: PlaygroundPrimitive::NoisyCrookCylinder(NoisySurface::from_params(
				crook, noise,
			)),
			res_2: helper.res_2,
			transform: helper.preview_transform(),
		};
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
