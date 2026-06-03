use bevy::prelude::*;

use crate::commands::render::TemperateConiferRenderHelper;
use crate::render::{RenderConfig, RenderSubject};

pub struct TemperateConiferRenderPlugin;

impl Plugin for TemperateConiferRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_temperate_conifer);
	}
}

pub fn react_render_helper_temperate_conifer(
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &TemperateConiferRenderHelper), Added<TemperateConiferRenderHelper>>,
) {
	for (_entity, helper) in &q {
		config.subject = RenderSubject::TemperateConifer(helper.inner.clone());
	}
}
