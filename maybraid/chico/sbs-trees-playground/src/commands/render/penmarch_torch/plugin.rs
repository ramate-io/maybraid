use bevy::prelude::*;

use crate::commands::render::{PenmarchTorchRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct PenmarchTorchRenderPlugin;

impl Plugin for PenmarchTorchRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_penmarch_torch);
	}
}

pub fn react_render_helper_penmarch_torch(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &PenmarchTorchRenderHelper), Added<PenmarchTorchRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::PenmarchTorch(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
