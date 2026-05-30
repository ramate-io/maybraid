use bevy::prelude::*;

use crate::commands::render::{JungleGrowthRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct JungleGrowthRenderPlugin;

impl Plugin for JungleGrowthRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_jungle_growth);
	}
}

pub fn react_render_helper_jungle_growth(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &JungleGrowthRenderHelper), Added<JungleGrowthRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::JungleGrowth(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
