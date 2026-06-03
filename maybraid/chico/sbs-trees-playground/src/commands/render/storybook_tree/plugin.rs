use bevy::prelude::*;

use crate::commands::render::{Render, StorybookTreeRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct StorybookTreeRenderPlugin;

impl Plugin for StorybookTreeRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_storybook_tree);
	}
}

pub fn react_render_helper_storybook_tree(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &StorybookTreeRenderHelper), Added<StorybookTreeRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::StorybookTree(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
