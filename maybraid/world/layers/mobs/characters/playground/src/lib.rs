//! Interactive viewer for character clips on a clothed species.

mod animation;
pub mod camera;
pub mod character;
pub mod commands;
mod capture;
mod skinning;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use animation::{apply_joint_preview, draw_authoring_gizmos, AnimationPlayback};
use bevy::prelude::*;
use camera_controls::look::CameraLookPlugin;
use character::CharacterConfig;
use characters::{CharacterHostSystems, CharacterHostsPlugin, CharacterMotionSystems};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::GameCommandDrawerConfig;
use skinning::{dump_bones_to_console, DumpBonesRequest};

pub struct CharactersPlaygroundPlugin;

impl Plugin for CharactersPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CharacterConfig>()
			.init_resource::<character::CharacterSyncState>()
			.init_resource::<AnimationPlayback>()
			.init_resource::<DumpBonesRequest>()
			.init_resource::<capture::PendingFaceCapture>()
			.add_plugins(CameraLookPlugin::default())
			.add_plugins(CharacterHostsPlugin)
			.add_plugins(game_command_plugin())
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					setup_lighting,
					capture::apply_capture_lighting,
					capture::queue_face_capture_from_env,
				)
					.chain(),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					character::sync_character
						.after(capture_command_line_input::<PlaygroundCommand>),
					character::stamp_anim
						.after(character::sync_character)
						.after(CharacterHostSystems::Membership)
						.before(CharacterMotionSystems::Anim),
					character::drive_playback
						.after(character::stamp_anim)
						.before(CharacterMotionSystems::Anim),
					apply_joint_preview.after(CharacterMotionSystems::Anim),
					draw_authoring_gizmos.after(apply_joint_preview),
					dump_bones_to_console,
					capture::ensure_capture_ui_hidden,
					capture::apply_pending_face_capture.after(character::sync_character),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn game_command_plugin() -> GameCommandPlugin<PlaygroundCommand> {
	let mut plugin = GameCommandPlugin::with_config(ui::ui_config());
	if capture::capture_mode() {
		plugin = plugin.with_drawer_config(GameCommandDrawerConfig {
			open_at_start: false,
			..GameCommandDrawerConfig::default()
		});
	}
	plugin
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
