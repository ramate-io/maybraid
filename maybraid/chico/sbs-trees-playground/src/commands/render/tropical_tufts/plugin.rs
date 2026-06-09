use bevy::prelude::*;

use crate::commands::render::{Render, TropicalTuftsRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct TropicalTuftsRenderPlugin;

impl Plugin for TropicalTuftsRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_tropical_tufts);
	}
}

pub fn react_render_helper_tropical_tufts(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &TropicalTuftsRenderHelper), Added<TropicalTuftsRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::TropicalTufts(helper.configured_tropical_tufts());
		config.res_2 = helper.res_2();
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
