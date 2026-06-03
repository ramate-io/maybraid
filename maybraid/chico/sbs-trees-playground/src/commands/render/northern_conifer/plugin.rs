use bevy::prelude::*;

use crate::commands::render::NorthernConiferRenderHelper;
use crate::render::{RenderConfig, RenderSubject};

pub struct NorthernConiferRenderPlugin;

impl Plugin for NorthernConiferRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_northern_conifer);
	}
}

pub fn react_render_helper_northern_conifer(
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &NorthernConiferRenderHelper), Added<NorthernConiferRenderHelper>>,
) {
	for (_entity, helper) in &q {
		config.subject = RenderSubject::NorthernConifer(helper.inner.clone());
	}
}
