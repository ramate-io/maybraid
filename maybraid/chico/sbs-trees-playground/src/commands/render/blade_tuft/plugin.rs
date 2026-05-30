use bevy::prelude::*;

use crate::commands::render::{BladeTuftRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct BladeTuftRenderPlugin;

impl Plugin for BladeTuftRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_blade_tuft);
	}
}

pub fn react_render_helper_blade_tuft(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &BladeTuftRenderHelper), Added<BladeTuftRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::BladeTuft(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
