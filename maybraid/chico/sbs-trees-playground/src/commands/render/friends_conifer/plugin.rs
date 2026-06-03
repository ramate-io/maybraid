use bevy::prelude::*;

use crate::commands::render::FriendsConiferRenderHelper;
use crate::render::{RenderConfig, RenderSubject};

pub struct FriendsConiferRenderPlugin;

impl Plugin for FriendsConiferRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, react_render_helper_friends_conifer);
	}
}

pub fn react_render_helper_friends_conifer(
	mut config: ResMut<RenderConfig>,
	q: Query<(Entity, &FriendsConiferRenderHelper), Added<FriendsConiferRenderHelper>>,
) {
	for (_entity, helper) in &q {
		config.subject = RenderSubject::FriendsConifer(helper.inner.clone());
	}
}
