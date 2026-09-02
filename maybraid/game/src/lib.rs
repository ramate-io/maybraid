//! Maybraid game executable: home shell over the world playground.

mod flow;
mod shell;

pub use flow::{GameFlow, HomeRoute, WorldPause};

use bevy::prelude::*;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::{MenuNav, MenuNavPad};
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{WorldGameplayEnabled, WorldPlugin};
use menu_components::{ActiveOverlayKey, MENU_CLEAR};
use menu_playground::{
	CharacterMenuState, CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin,
	request_show_character,
};
use menu_screens::{
	CreateCharacterPlugin, CreateCharacterReady, GameMode, HomeMenuChoice, HomeScreenPlugin,
	InGameMenuChoice, InGameScreenPlugin, SpinRevealScreen, request_show_create_character,
};
use std::path::{Path, PathBuf};

use crate::shell::{
	apply_shell_look, attach_preview_camera, detach_preview_camera, enter_characters, enter_home,
	enter_world, enter_world_menu, exit_world_menu, spawn_menu_ui_camera,
};

/// Crate-local asset directory (`maybraid/game/assets`).
pub fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(WorldPlugin::game())
			.insert_resource(WorldGameplayEnabled(false))
			.insert_resource(ClearColor(MENU_CLEAR))
			.init_state::<GameFlow>()
			.add_sub_state::<WorldPause>()
			.add_plugins((
				HomeScreenPlugin,
				InGameScreenPlugin,
				CreateCharacterPlugin,
				CharacterScreenPlugin,
				CharacterPreviewPlugin,
				MenuControllerPlugin,
			))
			.add_systems(Startup, spawn_menu_ui_camera)
			.add_systems(OnEnter(GameFlow::Home), (enter_home, apply_shell_look))
			.add_systems(
				OnEnter(GameFlow::Characters),
				(enter_characters, apply_shell_look, attach_preview_camera),
			)
			.add_systems(OnExit(GameFlow::Characters), detach_preview_camera)
			.add_systems(OnEnter(GameFlow::World), (enter_world, apply_shell_look))
			.add_systems(OnEnter(WorldPause::Playing), apply_shell_look)
			.add_systems(OnEnter(WorldPause::Menu), (enter_world_menu, apply_shell_look))
			.add_systems(OnExit(WorldPause::Menu), exit_world_menu)
			.add_systems(PostStartup, apply_shell_look)
			.add_systems(
				Update,
				(
					route_home_choice.run_if(in_state(GameFlow::Home)),
					route_in_game_choice.run_if(in_state(WorldPause::Menu)),
					open_create_character_hud.run_if(in_state(GameFlow::Characters)),
					character_back_to_home.run_if(in_state(GameFlow::Characters)),
					toggle_world_pause
						.after(CharacterControlSystems)
						.run_if(in_state(GameFlow::World)),
				),
			);
	}
}

fn route_home_choice(
	mut choices: MessageReader<HomeMenuChoice>,
	mut flow: ResMut<NextState<GameFlow>>,
	mut mode: ResMut<GameMode>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match HomeRoute::from_choice(choice) {
		HomeRoute::World { label } => {
			mode.label = String::from(label);
			flow.set(GameFlow::World);
		}
		HomeRoute::Characters => flow.set(GameFlow::Characters),
		HomeRoute::Unimplemented => {}
	}
}

fn route_in_game_choice(
	mut choices: MessageReader<InGameMenuChoice>,
	mut flow: ResMut<NextState<GameFlow>>,
) {
	if choices.read().any(|choice| *choice == InGameMenuChoice::Leave) {
		flow.set(GameFlow::Home);
	}
}

fn toggle_world_pause(
	pause: Res<State<WorldPause>>,
	mut next: ResMut<NextState<WorldPause>>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::InGameMenu)) {
		return;
	}
	match pause.get() {
		WorldPause::Playing => {
			gameplay.0 = false;
			next.set(WorldPause::Menu);
		}
		WorldPause::Menu => next.set(WorldPause::Playing),
	}
}

fn open_create_character_hud(
	mut ready: MessageReader<CreateCharacterReady>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut commands: Commands,
) {
	let Some(ready) = ready.read().last() else {
		return;
	};
	*menu_state = CharacterMenuState::for_create(ready.items.clone());
	request_show_character(&mut commands);
}

fn character_back_to_home(
	mut flow: ResMut<NextState<GameFlow>>,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	screens: Query<(), Or<(With<CharacterScreen>, With<SpinRevealScreen>)>>,
) {
	if screens.is_empty() || overlay.0.is_some() || !nav.just_pressed(MenuNav::Back) {
		return;
	}
	flow.set(GameFlow::Home);
}

#[cfg(test)]
mod tests {
	use super::assets_root;

	#[test]
	fn crate_assets_contain_barlow() {
		let font = assets_root().join("fonts/barlow/BarlowSemiCondensed-Regular.ttf");
		assert!(font.is_file(), "expected Barlow at {}", font.display());
	}
}
