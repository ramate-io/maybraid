use bevy::prelude::*;

use crate::commands::render::{Render, SpearTuftRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct SpearTuftRenderPlugin;

impl Plugin for SpearTuftRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_spear_tuft);
	}
}

pub fn react_render_helper_spear_tuft(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &SpearTuftRenderHelper), Added<SpearTuftRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::SpearTuft(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
