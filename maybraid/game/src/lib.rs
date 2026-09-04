//! Maybraid game executable: home shell over the world playground.

mod flow;
mod shell;

pub use flow::{GameFlow, HomeRoute, WorldPause};

use bevy::prelude::*;
use lod_first_load::FirstLoadStatus;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::MenuNavPad;
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{SpawnTerrainReady, WorldGameplayEnabled, WorldPlugin};
use menu_components::{consume_screen_back, ActiveOverlayKey, ScreenBackPressed, MENU_CLEAR};
use menu_playground::{
	CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin, CharacterSessionPlugin,
};
use menu_screens::{
	cancel_pending_create, request_show_gallery, CreateCharacterPlugin, GalleryScreen, GameMode,
	HomeMenuChoice, HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, LoadingExplainerText,
	LoadingProgress, LoadingScreenPlugin, SpinRevealScreen,
};
use std::path::{Path, PathBuf};

use crate::shell::{
	apply_shell_look, attach_preview_camera, detach_preview_camera, enter_characters, enter_home,
	enter_loading, enter_world, enter_world_menu, exit_world_menu, spawn_menu_ui_camera,
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
				LoadingScreenPlugin,
			))
			.add_systems(Startup, spawn_menu_ui_camera)
			.add_systems(
				OnEnter(GameFlow::Home),
				(enter_home, apply_shell_look, attach_preview_camera),
			)
			.add_systems(
				OnEnter(GameFlow::Characters),
				(enter_characters, apply_shell_look, attach_preview_camera),
			)
			.add_systems(OnExit(GameFlow::Characters), detach_preview_camera)
			.add_systems(
				OnEnter(GameFlow::LoadingWorld),
				(enter_loading, apply_shell_look, detach_preview_camera),
			)
			.add_systems(
				OnEnter(GameFlow::World),
				(enter_world, apply_shell_look, detach_preview_camera),
			)
			.add_systems(OnEnter(WorldPause::Playing), apply_shell_look)
			.add_systems(OnEnter(WorldPause::Menu), (enter_world_menu, apply_shell_look))
			.add_systems(OnExit(WorldPause::Menu), exit_world_menu)
			.add_systems(PostStartup, apply_shell_look)
			.add_systems(
				Update,
				(
					route_home_choice.run_if(in_state(GameFlow::Home)),
					update_world_loading.run_if(in_state(GameFlow::LoadingWorld)),
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
	status: Res<FirstLoadStatus>,
	terrain: Res<SpawnTerrainReady>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match HomeRoute::from_choice(choice) {
		HomeRoute::World { label } => {
			mode.label = String::from(label);
			flow.set(if status.settled && terrain.0 {
				GameFlow::World
			} else {
				GameFlow::LoadingWorld
			});
		}
		HomeRoute::Characters => flow.set(GameFlow::Characters),
		HomeRoute::Unimplemented => {}
	}
}

fn update_world_loading(
	status: Res<FirstLoadStatus>,
	terrain: Res<SpawnTerrainReady>,
	mut progress: MessageWriter<LoadingProgress>,
	mut explainer: MessageWriter<LoadingExplainerText>,
	mut flow: ResMut<NextState<GameFlow>>,
) {
	progress.write(LoadingProgress(if terrain.0 {
		status.progress
	} else {
		status.progress.min(0.95)
	}));
	let text = if !terrain.0 {
		"Preparing terrain collision…".to_string()
	} else if status.outstanding > 0 {
		format!("Streaming world… {} jobs remaining", status.outstanding)
	} else {
		"Settling the initial view…".to_string()
	};
	explainer.write(LoadingExplainerText(text));
	if terrain.0 && status.settled {
		flow.set(GameFlow::World);
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
	mut backs: MessageReader<ScreenBackPressed>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
) {
	if !consume_screen_back(&nav, overlay.0.is_some(), &mut backs) {
		return;
	}
	if !character.is_empty() {
		// Back discards unsaved HUD edits. Persist only happens from Save.
		request_show_gallery(&mut commands);
		return;
	}
	if !spin.is_empty() {
		cancel_pending_create(&mut commands);
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
