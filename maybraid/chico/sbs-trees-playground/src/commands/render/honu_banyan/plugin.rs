use bevy::prelude::*;

use crate::commands::render::HonuBanyanRenderHelper;
use crate::render::{RenderConfig, RenderSubject};

pub struct HonuBanyanRenderPlugin;

impl Plugin for HonuBanyanRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_honu_banyan);
	}
}

pub fn react_render_helper_honu_banyan(
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &HonuBanyanRenderHelper), Added<HonuBanyanRenderHelper>>,
) {
	for (_entity, helper) in &q {
		config.subject = RenderSubject::HonuBanyan(helper.inner.clone());
	}
}
