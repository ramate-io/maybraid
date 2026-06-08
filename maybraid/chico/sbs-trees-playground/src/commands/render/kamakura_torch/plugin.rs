use bevy::prelude::*;

use crate::commands::render::{KamakuraTorchRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct KamakuraTorchRenderPlugin;

impl Plugin for KamakuraTorchRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_kamakura_torch);
	}
}

pub fn react_render_helper_kamakura_torch(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &KamakuraTorchRenderHelper), Added<KamakuraTorchRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::KamakuraTorch(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
