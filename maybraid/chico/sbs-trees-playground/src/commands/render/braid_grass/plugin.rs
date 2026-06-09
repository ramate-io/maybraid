use bevy::prelude::*;

use crate::commands::render::{BraidGrassRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct BraidGrassRenderPlugin;

impl Plugin for BraidGrassRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_braid_grass);
	}
}

pub fn react_render_helper_braid_grass(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &BraidGrassRenderHelper), Added<BraidGrassRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::BraidGrass(helper.configured_braid_grass());
		config.res_2 = helper.res_2();
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
