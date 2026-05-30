use bevy::prelude::*;

use crate::commands::render::{BuddhaHandTuftRenderHelper, Render};
use crate::render::{RenderConfig, RenderSubject};

pub struct BuddhaHandTuftRenderPlugin;

impl Plugin for BuddhaHandTuftRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_buddha_hand_tuft);
	}
}

pub fn react_render_helper_buddha_hand_tuft(
	mut commands: Commands,
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &BuddhaHandTuftRenderHelper), Added<BuddhaHandTuftRenderHelper>>,
	render_q: Query<&Render, Added<Render>>,
) {
	for (entity, helper) in &q {
		config.subject = RenderSubject::BuddhaHandTuft(helper.inner.clone().into());
		config.res_2 = helper.res_2;
		config.transform = helper.render_transform();
		commands.entity(entity).despawn();
	}
	for render in &render_q {
		let _ = render;
		*config = render.into_render_config();
	}
}
