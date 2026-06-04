use bevy::prelude::*;

use crate::commands::render::{CommonHighBushRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct CommonHighBushRenderPlugin;

impl Plugin for CommonHighBushRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_common_high_bush);
	}
}

pub fn react_render_helper_common_high_bush(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &CommonHighBushRenderHelper), Added<CommonHighBushRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::CommonHighBush(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
