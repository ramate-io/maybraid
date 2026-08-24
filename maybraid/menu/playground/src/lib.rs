//! Command-driven playground for Maybraid HUD menu screens.

pub mod character;
pub mod commands;
mod loading_demo;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use crozon_character_ui_menus::CharacterMenu;
use game_commands::command::{CommandConsoleOutput, GameCommandPlugin};
use game_commands::ui::GameCommandDrawerConfig;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use menu_screens::{HomeMenuChoice, HomeScreenPlugin, LoadingScreenPlugin, LoadingScreenSystems};

use crate::character::{CharacterMenuState, CharacterScreenPlugin};

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
		.add_plugins((HomeScreenPlugin, LoadingScreenPlugin, CharacterScreenPlugin))
		.add_systems(Startup, setup_camera)
		.add_systems(
			Update,
			(
				echo_home_choice,
				echo_character_menu,
				loading_demo::run_loading_demo.before(LoadingScreenSystems::Apply),
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

fn echo_character_menu(
	mut events: MessageReader<CharacterMenuEvent<CharacterMenu>>,
	menu_state: Res<CharacterMenuState>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for event in events.read() {
		match event {
			CharacterMenuEvent::MenuUpdate(_) => {
				console.0 = format!("character: {}", menu_state.0.species.value.label());
			}
			CharacterMenuEvent::CameraFocus(focus) => {
				console.0 = format!("character focus: {focus:?}");
			}
		}
	}
}
