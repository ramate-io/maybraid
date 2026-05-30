use bevy::prelude::*;

use crate::commands::render::{LiamsConiferRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct LiamsConiferRenderPlugin;

impl Plugin for LiamsConiferRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_liams_conifer);
	}
}

pub fn react_render_helper_liams_conifer(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &LiamsConiferRenderHelper), Added<LiamsConiferRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::LiamsConifer(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
