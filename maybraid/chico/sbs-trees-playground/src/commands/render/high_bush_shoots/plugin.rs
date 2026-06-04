use bevy::prelude::*;

use crate::commands::render::{HighBushShootsRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct HighBushShootsRenderPlugin;

impl Plugin for HighBushShootsRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_high_bush_shoots);
	}
}

pub fn react_render_helper_high_bush_shoots(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &HighBushShootsRenderHelper), Added<HighBushShootsRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::HighBushShoots(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
