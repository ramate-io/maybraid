//! Maybraid game executable: home shell over the world playground.

mod flow;
mod load;
mod shell;

pub use flow::{GameFlow, HomeRoute, PauseMenuRoute, WorldPause};

use bevy::prelude::*;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::MenuNavPad;
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{
	PlayerPhysicsEnabled, PlayerSpawnXz, TerrainStreamingEnabled, WorldGameplayEnabled,
	WorldMobHudEnabled, WorldPlayerLoadout, WorldPlugin, WorldSceneryVisible,
};
use menu_components::{consume_screen_back, ActiveOverlayKey, ScreenBackPressed, MENU_CLEAR};
use menu_playground::{
	ActiveCharacter, CharacterEditBaseline, CharacterEditorReturn, CharacterMenuState,
	CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin, CharacterSessionPlugin,
	EditingCharacter, RequestEditCharacter,
};
use menu_screens::{
	cancel_pending_create, request_show_gallery, request_show_in_game,
	request_show_in_game_settings, CreateCharacterPlugin, GalleryScreen, GameMode, HomeMenuChoice,
	HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, InGameSettings, InGameSettingsScreen,
	LoadingScreenPlugin, LoadingScreenSystems, MenuScreen, SpinRevealScreen,
};
use std::path::{Path, PathBuf};

use crate::shell::{
	apply_pause_character_look, apply_shell_look, attach_preview_camera, despawn_loading_backdrop,
	detach_preview_camera, enter_characters, enter_home, enter_loading_world, enter_world,
	enter_world_menu, exit_world_menu, restore_stashed_world_camera, spawn_loading_backdrop,
	stamp_preview_render_layers,
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
					crate::load::arm_first_load,
				),
			)
			.add_systems(
				OnExit(GameFlow::LoadingWorld),
				(despawn_loading_backdrop, crate::load::disarm_first_load),
			)
			.add_systems(
				OnEnter(GameFlow::World),
				(load_active_player_loadout, enter_world, apply_shell_look, detach_preview_camera)
					.chain(),
			)
			.add_systems(OnEnter(WorldPause::Playing), apply_shell_look)
			.add_systems(OnEnter(WorldPause::Menu), (enter_world_menu, apply_shell_look))
			.add_systems(OnExit(WorldPause::Menu), (exit_world_menu, restore_stashed_world_camera))
			.add_systems(
				PostStartup,
				(
					boot_shell,
					apply_shell_look,
					attach_preview_camera.run_if(not(starting_discovery_at_override)),
				)
					.chain(),
			)
			.add_systems(
				Update,
				(
					stamp_preview_render_layers,
					crate::load::finish_world_loading
						.run_if(in_state(GameFlow::LoadingWorld))
						.before(LoadingScreenSystems::Apply),
					route_home_choice.run_if(in_state(GameFlow::Home)),
					route_in_game_choice.run_if(in_state(WorldPause::Menu)),
					apply_pause_character_look.run_if(in_state(WorldPause::Menu)),
					sync_world_loadout_from_editor.run_if(in_state(WorldPause::Menu)),
					persist_changed_player_inventory,
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

fn starting_discovery_at_override(spawn: Res<PlayerSpawnXz>) -> bool {
	spawn.0.is_some()
}

fn boot_shell(
	spawn: Res<PlayerSpawnXz>,
	mut flow: ResMut<NextState<GameFlow>>,
	mut mode: ResMut<GameMode>,
	commands: Commands,
	screens: Query<Entity, With<MenuScreen>>,
) {
	if spawn.0.is_some() {
		mode.label = String::from("Discovery");
		enter_loading_world(commands, screens);
		flow.set(GameFlow::LoadingWorld);
		return;
	}
	enter_home(commands);
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
	mut edits: MessageWriter<RequestEditCharacter>,
	active: Option<Res<ActiveCharacter>>,
	loadout: Option<Res<WorldPlayerLoadout>>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match PauseMenuRoute::from_choice(choice) {
		PauseMenuRoute::Leave => flow.set(GameFlow::Home),
		PauseMenuRoute::Settings => request_show_in_game_settings(&mut commands),
		PauseMenuRoute::Character => {
			let Some(active) = active else {
				warn!("pause character: no active character");
				return;
			};
			edits.write(RequestEditCharacter {
				id: active.id,
				return_to: CharacterEditorReturn::InGame,
				inventory: loadout.map(|loadout| loadout.inventory.clone()),
			});
		}
		PauseMenuRoute::Stay => {}
	}
}

fn persist_changed_player_inventory(
	loadout: Option<Res<WorldPlayerLoadout>>,
	active: Option<Res<ActiveCharacter>>,
	save_root: Res<crozon_character_persist::SaveRoot>,
) {
	let Some(loadout) = loadout else {
		return;
	};
	if !loadout.is_changed() {
		return;
	}
	let Some(active) = active else {
		return;
	};
	if let Err(error) = crozon_inventory_user::save(&save_root, active.id, &loadout.inventory) {
		warn!("failed to persist inventory {}: {error}", active.id.to_hex());
	}
}

fn sync_world_loadout_from_editor(
	baseline: Option<Res<CharacterEditBaseline>>,
	editing: Option<Res<EditingCharacter>>,
	menu: Res<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut commands: Commands,
) {
	let Some(baseline) = baseline else {
		return;
	};
	if !baseline.is_changed() {
		return;
	}
	if return_to.as_deref() != Some(&CharacterEditorReturn::InGame) {
		return;
	}
	let Some(editing) = editing else {
		return;
	};
	commands.insert_resource(WorldPlayerLoadout::new(
		editing.id.to_hex(),
		menu.0.appearance(),
		menu.0.inventory.clone().unwrap_or_default(),
	));
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
	character: Query<(), With<CharacterScreen>>,
) {
	if settings.is_empty() && character.is_empty() {
		return;
	}
	if !consume_screen_back(nav.as_ref(), overlay.0.is_some(), &mut backs) {
		return;
	}
	commands.remove_resource::<CharacterEditorReturn>();
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
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use crozon_character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use crozon_character_model_user::CharacterModel;
	use crozon_character_persist::{CharacterId, SaveRoot};
	use crozon_characters::CharacterAppearance;
	use menu_playground::ActiveCharacter;

	use crate::{assets_root, persist_changed_player_inventory, read_player_loadout};
	use maybraid_world::WorldPlayerLoadout;

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

	#[test]
	fn persist_writes_the_live_bag() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		let model = CharacterModel::new(id, "Live", CharacterAppearance::default());
		crozon_character_model_user::save(&root, &model)?;
		crozon_inventory_user::save(&root, id, &Inventory::default())?;

		let bag = Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
		]);
		let mut world = World::new();
		world.insert_resource(root.clone());
		world.insert_resource(ActiveCharacter { id });
		world.insert_resource(WorldPlayerLoadout::new(
			id.to_hex(),
			CharacterAppearance::default(),
			bag.clone(),
		));
		world
			.run_system_once(persist_changed_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let loaded = crozon_inventory_user::load(&root, id)?;
		assert_eq!(loaded.items.len(), bag.items.len());
		assert_eq!(loaded.clothing, bag.clothing);
		assert_eq!(loaded.weapons, bag.weapons);
		Ok(())
	}
}
