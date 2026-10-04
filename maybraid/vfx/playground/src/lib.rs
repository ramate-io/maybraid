//! Isolated host for real-time VFX concepts.

mod camera;
pub mod commands;
mod ground;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use maybraid_vfx::VfxPlugin;

pub struct VfxPlaygroundPlugin;

impl Plugin for VfxPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(VfxPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()));
		app.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change.before(camera::camera_controller),
					camera::camera_controller,
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 180.0,
		color: Color::srgb(0.72, 0.78, 0.92),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 7200.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use game_commands::command::GameCommand;

	#[test]
	fn parses_help() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("help")?;
		assert!(matches!(command, PlaygroundCommand::Help));
		Ok(())
	}
}
