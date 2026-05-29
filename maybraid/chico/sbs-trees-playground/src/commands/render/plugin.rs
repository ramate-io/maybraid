mod announcer;

use bevy::prelude::*;

use super::blade_tuft::plugin::{
	react_render_helper_blade_tuft, BladeTuftRenderPlugin,
};
use super::liams_conifer::plugin::{react_render_helper_liams_conifer, LiamsConiferRenderPlugin};
use super::sopes_banyan::plugin::{react_render_helper_sopes_banyan, SopesBanyanRenderPlugin};
use super::succulent_tuft::plugin::{
	react_render_helper_succulent_tuft, SucculentTuftRenderPlugin,
};
use super::weeping_tuft::plugin::{react_render_helper_weeping_tuft, WeepingTuftRenderPlugin};

pub use announcer::despawn_render_command_announcer;

pub struct RenderCommandsPlugin;

impl Plugin for RenderCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			SopesBanyanRenderPlugin,
			LiamsConiferRenderPlugin,
			SucculentTuftRenderPlugin,
			BladeTuftRenderPlugin,
			WeepingTuftRenderPlugin,
		))
		.add_systems(
			Update,
			announcer::despawn_render_command_announcer
				.after(react_render_helper_sopes_banyan)
				.after(react_render_helper_liams_conifer)
				.after(react_render_helper_succulent_tuft)
				.after(react_render_helper_blade_tuft)
				.after(react_render_helper_weeping_tuft),
		);
	}
}
