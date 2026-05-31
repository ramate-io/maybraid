use bevy::prelude::*;

use crate::commands::render::{DatePalmRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct DatePalmRenderPlugin;

impl Plugin for DatePalmRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_date_palm);
	}
}

pub fn react_render_helper_date_palm(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &DatePalmRenderHelper), Added<DatePalmRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::DatePalm(helper.inner.clone());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
