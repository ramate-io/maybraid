use bevy::prelude::*;

use crate::commands::render::RorysHeadTrainedRenderHelper;
use crate::render::{RenderConfig, RenderSubject};

pub struct RorysHeadTrainedRenderPlugin;

impl Plugin for RorysHeadTrainedRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_rorys_head_trained);
	}
}

pub fn react_render_helper_rorys_head_trained(
	mut config: ResMut<RenderConfig>,
	mut commands: Commands,
	q: Query<(Entity, &RorysHeadTrainedRenderHelper), Added<RorysHeadTrainedRenderHelper>>,
) {
	for (entity, helper) in &q {
		commands.entity(entity).despawn();
		config.subject = RenderSubject::RorysHeadTrained(helper.inner.clone());
	}
}
