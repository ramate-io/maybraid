//! Maybraid game executable: home shell over the world playground.

mod flow;

pub use flow::{home_destination, world_mode_label, GameFlow};

use bevy::prelude::*;
use crozon_character_playground::CameraController as PreviewCameraController;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::{MenuNav, MenuNavPad};
use maybraid_menu_controller::{MenuController, MenuControllerPlugin};
use maybraid_world::{WorldGameplayEnabled, WorldPlugin};
use menu_components::{ActiveOverlayKey, TextMenuSystems};
use menu_playground::{
	request_show_character, CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin,
};
use menu_screens::{
	request_show_home, request_show_in_game, GameMode, HomeMenuChoice, HomeScreen,
	HomeScreenPlugin, InGameMenuChoice, InGameScreen, InGameScreenPlugin, MenuScreen,
};
use std::path::{Path, PathBuf};

const HOME_BACKDROP: Color = Color::srgb(0.08, 0.10, 0.14);

/// Shared Maybraid asset tree (`maybraid/assets`). This crate sits one level
/// shallower than `maybraid/*/playground`, so the relative path is `../assets`.
pub fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GameFlow>()
			.add_plugins(WorldPlugin)
			.add_plugins((
				HomeScreenPlugin,
				InGameScreenPlugin,
				CharacterScreenPlugin,
				CharacterPreviewPlugin,
				MenuControllerPlugin,
			))
			.add_systems(Startup, show_home)
			.add_systems(PostStartup, sync_world_gameplay)
			.add_systems(PreUpdate, attach_menu_controllers)
			.add_systems(
				Update,
				(
					route_home_choice,
					route_in_game_choice,
					character_back_to_home,
					overlay_in_game_from_start
						.after(CharacterControlSystems)
						.before(TextMenuSystems::Navigate),
					paint_home_backdrop,
					sync_preview_camera,
					sync_world_gameplay,
				),
			);
	}
}

fn show_home(mut commands: Commands) {
	request_show_home(&mut commands);
}

fn attach_menu_controllers(
	mut commands: Commands,
	screens: Query<Entity, (With<MenuScreen>, Without<MenuController>)>,
) {
	for entity in &screens {
		commands.entity(entity).insert(MenuController::default());
	}
}

fn paint_home_backdrop(
	mut commands: Commands,
	homes: Query<Entity, (With<HomeScreen>, Without<BackgroundColor>)>,
) {
	for entity in &homes {
		commands.entity(entity).insert(BackgroundColor(HOME_BACKDROP));
	}
}

fn route_home_choice(
	mut commands: Commands,
	mut choices: MessageReader<HomeMenuChoice>,
	mut flow: ResMut<GameFlow>,
	mut mode: ResMut<GameMode>,
	screens: Query<Entity, With<MenuScreen>>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match home_destination(choice) {
		Some(GameFlow::World) => {
			if let Some(label) = world_mode_label(choice) {
				mode.label = String::from(label);
			}
			despawn_menu_screens(&mut commands, &screens);
			*flow = GameFlow::World;
		}
		Some(GameFlow::Characters) => {
			request_show_character(&mut commands);
			*flow = GameFlow::Characters;
		}
		Some(GameFlow::Home) | None => {}
	}
}

fn route_in_game_choice(
	mut commands: Commands,
	mut choices: MessageReader<InGameMenuChoice>,
	mut flow: ResMut<GameFlow>,
) {
	for choice in choices.read() {
		if *choice == InGameMenuChoice::Leave {
			*flow = GameFlow::Home;
			request_show_home(&mut commands);
		}
	}
}

fn overlay_in_game_from_start(
	mut commands: Commands,
	flow: Res<GameFlow>,
	mut intents: MessageReader<CharacterIntent>,
	mut nav: ResMut<MenuNavPad>,
	overlay: Query<Entity, With<InGameScreen>>,
) {
	if *flow != GameFlow::World {
		return;
	}
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::InGameMenu)) {
		return;
	}
	nav.events.retain(|event| *event != MenuNav::Select);
	if overlay.is_empty() {
		request_show_in_game(&mut commands);
	} else {
		for entity in &overlay {
			commands.entity(entity).despawn();
		}
	}
}

fn character_back_to_home(
	mut commands: Commands,
	mut flow: ResMut<GameFlow>,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	screens: Query<(), With<CharacterScreen>>,
) {
	if *flow != GameFlow::Characters || screens.is_empty() {
		return;
	}
	if overlay.0.is_some() || !nav.just_pressed(MenuNav::Back) {
		return;
	}
	request_show_home(&mut commands);
	*flow = GameFlow::Home;
}

fn sync_world_gameplay(
	flow: Res<GameFlow>,
	overlay: Query<(), With<InGameScreen>>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
) {
	gameplay.0 = *flow == GameFlow::World && overlay.is_empty();
}

fn sync_preview_camera(
	mut commands: Commands,
	flow: Res<GameFlow>,
	cameras: Query<Entity, (With<Camera3d>, Without<PreviewCameraController>)>,
	preview: Query<Entity, (With<Camera3d>, With<PreviewCameraController>)>,
) {
	if *flow == GameFlow::Characters {
		for entity in &cameras {
			commands.entity(entity).insert(PreviewCameraController {
				speed: 6.0,
				sensitivity: 0.005,
				yaw: 0.0,
				pitch: 0.0,
			});
		}
	} else {
		for entity in &preview {
			commands.entity(entity).remove::<PreviewCameraController>();
		}
	}
}

fn despawn_menu_screens(commands: &mut Commands, screens: &Query<Entity, With<MenuScreen>>) {
	for entity in screens {
		commands.entity(entity).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::assets_root;

	#[test]
	fn assets_root_points_at_maybraid_assets() {
		let font = assets_root().join("fonts/barlow/BarlowSemiCondensed-Regular.ttf");
		assert!(font.is_file(), "expected Barlow at {}", font.display());
	}
}
