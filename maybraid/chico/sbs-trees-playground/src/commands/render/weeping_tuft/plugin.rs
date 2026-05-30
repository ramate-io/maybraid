use bevy::prelude::*;

use crate::commands::render::{Render, WeepingTuftRenderHelper};
use crate::render::{RenderConfig, RenderSubject};

pub struct WeepingTuftRenderPlugin;

impl Plugin for WeepingTuftRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_weeping_tuft);
	}
}

pub fn react_render_helper_weeping_tuft(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &WeepingTuftRenderHelper), Added<WeepingTuftRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::WeepingTuft(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
