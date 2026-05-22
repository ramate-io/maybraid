use bevy::prelude::*;

use crate::commands::render::{Render, LiamsConiferRenderHelper};
use crate::preview::{PreviewConfig, PreviewTree};

pub struct LiamsConiferRenderPlugin;

impl Plugin for LiamsConiferRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_liams_conifer);
	}
}

pub fn react_render_helper_liams_conifer(
	mut commands: Commands,
	mut config: ResMut<PreviewConfig>,
	q: Query<(Entity, &LiamsConiferRenderHelper), Added<LiamsConiferRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.tree = PreviewTree::LiamsConifer(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.preview_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_preview_config();
	}
}
