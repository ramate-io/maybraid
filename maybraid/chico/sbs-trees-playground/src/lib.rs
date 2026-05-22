//! Interactive viewer for Chico stalk-and-ball-stick trees.

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod preview;
mod preview_materials;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PlaygroundCommandsPlugin, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::{PreviewConfig, SbsPreviewRoot};

use bevy::prelude::*;
use chico_sbs_trees::liams_conifer::render_item_plugin::LiamsConiferRenderItemPlugin;
use chico_sbs_trees::sopes_banyan::render_item_plugin::SopesBanyanRenderItemPlugin;
use chico_sdf::{NoisyBall, NoisyCylinder};
use chico_vegetation_shaders::{
	ChicoLeafMaterial, ChicoStickMaterial, ChicoVegetationShadersPlugin,
};
use commands::render::liams_conifer::plugin::react_render_helper_liams_conifer;
use commands::render::sopes_banyan::plugin::react_render_helper_sopes_banyan;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use preview::sync_tree_preview;
use preview_materials::{setup_preview_tree_materials, sync_preview_tree_material_handles};
use render_item::mesh::handle::EnforceCachingPlugin;
use render_item::render_items;

pub struct SbsTreesPlaygroundPlugin;

impl Plugin for SbsTreesPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.add_plugins((
				SopesBanyanRenderItemPlugin::default(),
				LiamsConiferRenderItemPlugin::default(),
			))
			.add_plugins(ChicoVegetationShadersPlugin)
			.add_plugins(EnforceCachingPlugin::<NoisyCylinder, ChicoStickMaterial>::default())
			.add_plugins(EnforceCachingPlugin::<NoisyBall, ChicoLeafMaterial>::default())
			.add_plugins(PlaygroundCommandsPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(
				Startup,
				(camera::setup_camera, setup_lighting, setup_ground, setup_preview_tree_materials),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					sync_preview_tree_material_handles
						.after(react_render_helper_sopes_banyan)
						.after(react_render_helper_liams_conifer)
						.before(sync_tree_preview),
					sync_tree_preview.after(sync_preview_tree_material_handles),
					(
						render_items::<crate::preview::PreviewSopesBanyan>,
						render_items::<crate::preview::PreviewLiamsConifer>,
					)
						.after(sync_tree_preview),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadows_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
