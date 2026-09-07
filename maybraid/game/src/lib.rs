//! Maybraid game executable: home shell over the world playground.

mod flow;
mod shell;

pub use flow::{GameFlow, HomeRoute, PauseMenuRoute, WorldPause};

use bevy::prelude::*;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::MenuNavPad;
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{
	PlayerPhysicsEnabled, TerrainStreamingEnabled, WorldGameplayEnabled, WorldMobHudEnabled,
	WorldPlayerLoadout, WorldPlugin, WorldSceneryVisible, WorldSurfaceReady,
};
use menu_components::{consume_screen_back, ActiveOverlayKey, ScreenBackPressed, MENU_CLEAR};
use menu_playground::{
	ActiveCharacter, CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin,
	CharacterSessionPlugin,
};
use menu_screens::{
	cancel_pending_create, request_show_gallery, request_show_in_game,
	request_show_in_game_settings, CreateCharacterPlugin, GalleryScreen, GameMode, HomeMenuChoice,
	HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, InGameSettings, InGameSettingsScreen,
	LoadingScreenPlugin, SpinRevealScreen,
};
use std::path::{Path, PathBuf};

use crate::shell::{
	apply_shell_look, attach_preview_camera, despawn_loading_backdrop, detach_preview_camera,
	enter_characters, enter_home, enter_loading_world, enter_world, enter_world_menu,
	exit_world_menu, spawn_loading_backdrop, stamp_preview_render_layers,
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
			.insert_resource(PlayerPhysicsEnabled(false))
			.insert_resource(TerrainStreamingEnabled(false))
			.insert_resource(WorldSceneryVisible(false))
			.insert_resource(ClearColor(MENU_CLEAR))
			.init_state::<GameFlow>()
			.add_sub_state::<WorldPause>()
			.add_plugins((
				HomeScreenPlugin,
				InGameScreenPlugin,
				LoadingScreenPlugin,
				CreateCharacterPlugin,
				CharacterSessionPlugin,
				CharacterScreenPlugin,
				CharacterPreviewPlugin,
				MenuControllerPlugin,
			))
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
				(
					enter_loading_world,
					spawn_loading_backdrop,
					apply_shell_look,
					detach_preview_camera,
				),
			)
			.add_systems(OnExit(GameFlow::LoadingWorld), despawn_loading_backdrop)
			.add_systems(
				OnEnter(GameFlow::World),
				(load_active_player_loadout, enter_world, apply_shell_look, detach_preview_camera)
					.chain(),
			)
			.add_systems(OnEnter(WorldPause::Playing), apply_shell_look)
			.add_systems(OnEnter(WorldPause::Menu), (enter_world_menu, apply_shell_look))
			.add_systems(OnExit(WorldPause::Menu), exit_world_menu)
			.add_systems(PostStartup, (enter_home, apply_shell_look, attach_preview_camera))
			.add_systems(
				Update,
				(
					stamp_preview_render_layers,
					finish_world_loading.run_if(in_state(GameFlow::LoadingWorld)),
					route_home_choice.run_if(in_state(GameFlow::Home)),
					route_in_game_choice.run_if(in_state(WorldPause::Menu)),
					sync_world_mob_hud,
					pause_menu_back.run_if(in_state(WorldPause::Menu)),
					character_back.run_if(in_state(GameFlow::Characters)),
					toggle_world_pause
						.after(CharacterControlSystems)
						.run_if(in_state(GameFlow::World)),
				),
			);
	}
}

fn finish_world_loading(ready: Res<WorldSurfaceReady>, mut flow: ResMut<NextState<GameFlow>>) {
	if ready.0 {
		flow.set(GameFlow::World);
	}
}

fn load_active_player_loadout(
	mut commands: Commands,
	active: Option<Res<ActiveCharacter>>,
	save_root: Res<crozon_character_persist::SaveRoot>,
) {
	commands.remove_resource::<WorldPlayerLoadout>();
	let Some(active) = active else {
		warn!("entering world without an active character; using the default world loadout");
		return;
	};
	let loadout = match read_player_loadout(save_root.as_ref(), active.id) {
		Ok(loadout) => loadout,
		Err(error) => {
			warn!("failed to load active player {} for world: {error}", active.id.to_hex());
			return;
		}
	};
	commands.insert_resource(loadout);
}

fn read_player_loadout(
	save_root: &crozon_character_persist::SaveRoot,
	id: crozon_character_persist::CharacterId,
) -> Result<WorldPlayerLoadout, crozon_character_persist::PersistError> {
	let model = crozon_character_model_user::load(save_root, id)?;
	let inventory = crozon_inventory_user::load(save_root, id)?;
	Ok(WorldPlayerLoadout::new(id.to_hex(), model.appearance, inventory))
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
			flow.set(GameFlow::LoadingWorld);
		}
		HomeRoute::Characters => flow.set(GameFlow::Characters),
		HomeRoute::Unimplemented => {}
	}
}

fn route_in_game_choice(
	mut choices: MessageReader<InGameMenuChoice>,
	mut flow: ResMut<NextState<GameFlow>>,
	mut commands: Commands,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match PauseMenuRoute::from_choice(choice) {
		PauseMenuRoute::Leave => flow.set(GameFlow::Home),
		PauseMenuRoute::Settings => request_show_in_game_settings(&mut commands),
		PauseMenuRoute::Stay => {}
	}
}

fn sync_world_mob_hud(settings: Res<InGameSettings>, mut hud: ResMut<WorldMobHudEnabled>) {
	if hud.0 != settings.mob_hud {
		hud.0 = settings.mob_hud;
	}
}

fn pause_menu_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	mut backs: MessageReader<ScreenBackPressed>,
	settings: Query<(), With<InGameSettingsScreen>>,
) {
	if settings.is_empty() {
		return;
	}
	if !consume_screen_back(nav.as_ref(), overlay.0.is_some(), &mut backs) {
		return;
	}
	request_show_in_game(&mut commands);
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
	if !consume_screen_back(nav.as_ref(), overlay.0.is_some(), &mut backs) {
		return;
	}
	if !character.is_empty() {
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
	use crozon_character_items::Inventory;
	use crozon_character_model_user::CharacterModel;
	use crozon_character_persist::{CharacterId, SaveRoot};
	use crozon_characters::CharacterAppearance;

	use crate::{assets_root, read_player_loadout};

	#[test]
	fn crate_assets_contain_barlow() {
		let font = assets_root().join("fonts/barlow/BarlowSemiCondensed-Regular.ttf");
		assert!(font.is_file(), "expected Barlow at {}", font.display());
	}

	#[test]
	fn discovery_reads_the_active_character_files() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(42);
		let model = CharacterModel::new(id, "Active", CharacterAppearance::default());
		let inventory = Inventory::default();
		crozon_character_model_user::save(&root, &model)?;
		crozon_inventory_user::save(&root, id, &inventory)?;

		let loadout = read_player_loadout(&root, id)?;
		assert_eq!(loadout.key, id.to_hex());
		assert_eq!(loadout.inventory, inventory);
		assert_eq!(loadout.appearance.species_id(), model.appearance.species_id());
		Ok(())
	}
}
