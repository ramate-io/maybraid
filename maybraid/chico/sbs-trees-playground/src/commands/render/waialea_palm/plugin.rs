use bevy::prelude::*;

use crate::commands::render::{Render, WaialeaPalmRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct WaialeaPalmRenderPlugin;

impl Plugin for WaialeaPalmRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_waialea_palm);
	}
}

pub fn react_render_helper_waialea_palm(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &WaialeaPalmRenderHelper), Added<WaialeaPalmRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::WaialeaPalm(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
