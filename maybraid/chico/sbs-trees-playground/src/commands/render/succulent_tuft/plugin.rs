use bevy::prelude::*;

use crate::commands::render::{Render, SucculentTuftRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct SucculentTuftRenderPlugin;

impl Plugin for SucculentTuftRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_succulent_tuft);
	}
}

pub fn react_render_helper_succulent_tuft(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &SucculentTuftRenderHelper), Added<SucculentTuftRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::SucculentTuft(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
