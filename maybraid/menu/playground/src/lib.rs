//! Command-driven playground for Maybraid HUD menu screens.

pub mod character;
pub mod commands;
mod loading_demo;
mod preview;
mod ui;

pub use character::{
	request_show_character, CharacterMenuState, CharacterScreen, CharacterScreenPlugin,
	RequestShowCharacter,
};
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::CharacterPreviewPlugin;

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use crozon_character_playground::camera;
use crozon_character_ui_menus::MenuEvent;
use crozon_characters::CharacterHostsPlugin;
use game_commands::command::{CommandConsoleOutput, GameCommandPlugin};
use game_commands::ui::GameCommandDrawerConfig;
use lod::LodViewer;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use maybraid_input::{VirtualPadConfig, VirtualPadPlugin};
use maybraid_menu_controller::{MenuController, MenuControllerPlugin};
use menu_screens::{
	HomeMenuChoice, HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, LoadingScreenPlugin,
	LoadingScreenSystems, MenuScreen,
};

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
		.add_plugins(CharacterHostsPlugin)
		.add_plugins(CameraLookPlugin::new(CameraLookConfig {
			enabled_at_start: false,
			toggle_keys: Vec::new(),
			..CameraLookConfig::default()
		}))
		.add_plugins(VirtualPadPlugin::new(VirtualPadConfig { debug_overlay: true, ..default() }))
		.add_plugins((
			HomeScreenPlugin,
			InGameScreenPlugin,
			LoadingScreenPlugin,
			CharacterScreenPlugin,
			CharacterPreviewPlugin,
			MenuControllerPlugin,
		))
		.add_systems(
			Startup,
			(camera::setup_camera, add_lod_viewer_to_camera.after(camera::setup_camera)),
		)
		.add_systems(PreUpdate, attach_menu_controllers)
		.add_systems(
			Update,
			(
				camera::camera_controller.run_if(character_screen_closed),
				echo_home_choice,
				echo_in_game_choice,
				echo_character_menu,
				loading_demo::run_loading_demo.before(LoadingScreenSystems::Apply),
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
	}
}

fn add_lod_viewer_to_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<LodViewer>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(LodViewer);
	}
}

fn character_screen_closed(screens: Query<(), With<CharacterScreen>>) -> bool {
	screens.is_empty()
}

fn attach_menu_controllers(
	mut commands: Commands,
	screens: Query<Entity, (With<MenuScreen>, Without<MenuController>)>,
) {
	for entity in &screens {
		commands.entity(entity).insert(MenuController::default());
	}
}

fn echo_home_choice(
	mut choices: MessageReader<HomeMenuChoice>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for choice in choices.read() {
		console.0 = format!("home: {}", choice.label());
	}
}

fn echo_in_game_choice(
	mut choices: MessageReader<InGameMenuChoice>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for choice in choices.read() {
		console.0 = format!("in-game: {}", choice.label());
	}
}

fn echo_character_menu(
	mut events: MessageReader<CharacterMenuEvent<MenuEvent>>,
	menu_state: Res<CharacterMenuState>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for event in events.read() {
		match event {
			CharacterMenuEvent::Menu(_) => {
				console.0 = format!("character: {}", menu_state.0.species.value.label());
			}
			CharacterMenuEvent::CameraFocus(focus) => {
				console.0 = format!("character focus: {focus:?}");
			}
		}
	}
}
