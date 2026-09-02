//! Maybraid game executable: home shell over the world playground.

mod flow;
mod shell;

pub use flow::{GameFlow, HomeRoute, WorldPause};

use bevy::prelude::*;
use crozon_character_persist::SaveRoot;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::{MenuNav, MenuNavPad};
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{WorldGameplayEnabled, WorldPlugin};
use menu_components::{ActiveOverlayKey, MENU_CLEAR};
use menu_playground::{
	save_editing_character, CharacterMenuState, CharacterPreviewPlugin, CharacterScreen,
	CharacterScreenPlugin, CharacterSessionPlugin, EditingCharacter,
};
use menu_screens::{
	request_show_gallery, CreateCharacterPlugin, GalleryScreen, GameMode, HomeMenuChoice,
	HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, SpinRevealScreen,
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
				CharacterSessionPlugin,
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
					character_back.run_if(in_state(GameFlow::Characters)),
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

fn character_back(
	mut flow: ResMut<NextState<GameFlow>>,
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
	save_root: Res<SaveRoot>,
	editing: Option<Res<EditingCharacter>>,
	menu_state: Res<CharacterMenuState>,
) {
	if overlay.0.is_some() || !nav.just_pressed(MenuNav::Back) {
		return;
	}
	if !character.is_empty() {
		if let Some(editing) = editing.as_ref() {
			if let Err(error) = save_editing_character(&save_root, editing.id, &menu_state.0) {
				warn!("failed to save character {}: {error}", editing.id.to_hex());
			}
		}
		request_show_gallery(&mut commands);
		return;
	}
	if !spin.is_empty() {
		request_show_gallery(&mut commands);
		return;
	}
	if !gallery.is_empty() {
		flow.set(GameFlow::Home);
	}
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
