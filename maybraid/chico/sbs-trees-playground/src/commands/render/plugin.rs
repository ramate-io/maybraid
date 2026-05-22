mod announcer;

use bevy::prelude::*;

use super::liams_conifer::plugin::{react_render_helper_liams_conifer, LiamsConiferRenderPlugin};
use super::sopes_banyan::plugin::{react_render_helper_sopes_banyan, SopesBanyanRenderPlugin};

pub use announcer::despawn_render_command_announcer;

pub struct RenderCommandsPlugin;

impl Plugin for RenderCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((SopesBanyanRenderPlugin, LiamsConiferRenderPlugin)).add_systems(
			Update,
			announcer::despawn_render_command_announcer
				.after(react_render_helper_sopes_banyan)
				.after(react_render_helper_liams_conifer),
		);
	}
}
