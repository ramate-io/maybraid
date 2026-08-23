//! Command-driven playground for Maybraid HUD menu screens.

pub mod commands;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use game_commands::command::{CommandConsoleOutput, GameCommandPlugin};
use game_commands::ui::GameCommandDrawerConfig;
use menu_screens::{HomeMenuChoice, HomeScreenPlugin};

pub struct MenuPlaygroundPlugin;

impl Plugin for MenuPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
				.with_drawer_config(GameCommandDrawerConfig {
					open_at_start: false,
					toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
					..default()
				}),
		)
		.add_plugins(HomeScreenPlugin)
		.add_systems(Startup, setup_camera)
		.add_systems(
			Update,
			(
				echo_home_choice,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
	}
}

fn setup_camera(mut commands: Commands) {
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 1.6, 3.5).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
	));
}

fn echo_home_choice(
	mut choices: MessageReader<HomeMenuChoice>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for choice in choices.read() {
		console.0 = format!("home: {}", choice.label());
	}
}
