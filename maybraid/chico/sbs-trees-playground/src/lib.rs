//! Interactive viewer for Chico stalk-and-ball-stick trees.

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod input;
mod preview;
mod startup;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PlaygroundCommandsPlugin, PLAYGROUND_CLI_NAME};
pub use preview::{PreviewConfig, SbsPreviewRoot};
pub use startup::PendingStartupCommand;

use bevy::prelude::*;
use chico_sbs_trees::sopes_banyan::render_item_plugin::SopesBanyanRenderItemPlugin;
use commands::root::react_playground_command_root;
use game_commands::ui::GameCommandUiPlugin;
use ground::setup_ground;
use preview::sync_tree_preview;
use render_item::render_items;

pub struct SbsTreesPlaygroundPlugin;

impl Plugin for SbsTreesPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.init_resource::<PendingStartupCommand>()
			.add_plugins(SopesBanyanRenderItemPlugin::default())
			.add_plugins(PlaygroundCommandsPlugin)
			.add_plugins(GameCommandUiPlugin { config: ui::ui_config() })
			.add_plugins(startup::StartupPlugin)
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.init_resource::<input::TypedCommandLine>()
			.init_resource::<input::TextEntryFocus>()
			.init_resource::<input::CommandConsoleOutput>()
			.init_resource::<input::CommandHistory>()
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					input::toggle_text_entry_focus,
					input::capture_command_line_input,
					sync_tree_preview.after(react_playground_command_root),
					ui::update_debug_ui,
					render_items::<chico_sbs_trees::sopes_banyan::SopesBanyan>,
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
