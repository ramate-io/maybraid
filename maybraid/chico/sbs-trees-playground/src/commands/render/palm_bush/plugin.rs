use bevy::prelude::*;

use crate::commands::render::PalmBushRenderHelper;
use crate::render::RenderSubject;

pub struct PalmBushRenderPlugin;

impl Plugin for PalmBushRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_palm_bush);
	}
}

pub fn react_render_helper_palm_bush(
	mut commands: Commands,
	mut config: ResMut<crate::render::RenderConfig>,
	q: Query<(Entity, &PalmBushRenderHelper), Added<PalmBushRenderHelper>>,
) {
	for (entity, helper) in &q {
		commands.entity(entity).despawn();
		config.subject = RenderSubject::PalmBush(helper.inner.clone());
	}
}
