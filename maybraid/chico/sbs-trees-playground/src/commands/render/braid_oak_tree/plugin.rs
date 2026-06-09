use bevy::prelude::*;

use crate::commands::render::{BraidOakTreeRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct BraidOakTreeRenderPlugin;

impl Plugin for BraidOakTreeRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_braid_oak_tree);
	}
}

pub fn react_render_helper_braid_oak_tree(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &BraidOakTreeRenderHelper), Added<BraidOakTreeRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::BraidOakTree(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
