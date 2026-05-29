use bevy::prelude::*;

use crate::commands::render::{Render, SopesBanyanRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct SopesBanyanRenderPlugin;

impl Plugin for SopesBanyanRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_sopes_banyan);
	}
}

pub fn react_render_helper_sopes_banyan(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &SopesBanyanRenderHelper), Added<SopesBanyanRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::SopesBanyan(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
