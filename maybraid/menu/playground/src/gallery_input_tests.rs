//! Drive gallery Select / Back / Edit / Delete through the real pad and keyboard path.

use anyhow::{Context as ContextExt, Result};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::{ButtonState, InputPlugin};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use character_persist::{CharacterId, SaveRoot};
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_virtual_pad_controller::PadHidPolicy;
use menu_components::{
	GalleryPromptKind, MenuModalId, MenuModalStack, ScreenBack, ScreenEdit, TextMenu,
};
use menu_screens::{
	request_show_gallery, GalleryChoice, GalleryScreen, RequestShowCreateCharacter, RequestShowHome,
};

use crate::character::RequestShowCharacter;
use crate::session::{
	save_editing_character, ActiveCharacter, CharacterSessionPlugin, EditingCharacter,
};
use crate::CharacterMenuState;

fn scratch_root() -> SaveRoot {
	SaveRoot::at(std::env::temp_dir().join(format!(
		"maybraid-gallery-input-{}-{}",
		std::process::id(),
		CharacterId::new().to_hex()
	)))
}

fn save_named(root: &SaveRoot, id: CharacterId, name: &str) -> Result<()> {
	let mut menu = CharacterMenuState::default();
	menu.0.name = name.into();
	save_editing_character(root, id, &menu.0)?;
	Ok(())
}

fn gallery_app(root: SaveRoot, active: CharacterId) -> Result<App> {
	character_persist::save_active(&root, active)?;
	let mut app = App::new();
	app.insert_resource(PadHidPolicy::External);
	app.add_plugins((
		MinimalPlugins,
		InputPlugin,
		AssetPlugin::default(),
		ScenePlugin,
		bevy::window::WindowPlugin { primary_window: None, ..default() },
	));
	app.init_asset::<Font>().init_asset::<Image>();
	app.init_resource::<bevy::picking::hover::HoverMap>();
	app.insert_resource(root);
	app.init_resource::<CharacterMenuState>();
	app.add_plugins((MenuControllerPlugin, CharacterSessionPlugin));
	app.add_systems(
		Update,
		crate::editor_back
			.after(menu_components::TextMenuSystems::Navigate)
			.after(maybraid_controllers_core::PadDeliverySystems),
	);
	let gamepad = app.world_mut().spawn(Gamepad::default()).id();
	app.insert_resource(TestGamepad(gamepad));
	request_show_gallery(&mut app.world_mut().commands());
	app.world_mut().flush();
	app.update();
	app.update();
	anyhow::ensure!(
		app.world_mut()
			.query_filtered::<(), With<GalleryScreen>>()
			.iter(app.world())
			.next()
			.is_some(),
		"gallery screen should spawn"
	);
	Ok(app)
}

#[derive(Resource, Clone, Copy)]
struct TestGamepad(Entity);

fn gamepad_button(app: &mut App, button: GamepadButton, down: bool) -> Result<()> {
	let gamepad = app.world().resource::<TestGamepad>().0;
	let mut device = app.world_mut().get_mut::<Gamepad>(gamepad).context("gamepad")?;
	if down {
		device.digital_mut().press(button);
	} else {
		device.digital_mut().release(button);
	}
	app.update();
	Ok(())
}

fn tap_pad(app: &mut App, button: GamepadButton) -> Result<()> {
	gamepad_button(app, button, true)?;
	gamepad_button(app, button, false)?;
	for _ in 0..3 {
		app.update();
	}
	Ok(())
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
	app.world_mut().write_message(KeyboardInput {
		key_code,
		logical_key: Key::Unidentified(NativeKey::Unidentified),
		state,
		text: None,
		repeat: false,
		window: Entity::PLACEHOLDER,
	});
}

fn tap_key(app: &mut App, key_code: KeyCode) {
	key(app, key_code, ButtonState::Pressed);
	app.update();
	key(app, key_code, ButtonState::Released);
	for _ in 0..3 {
		app.update();
	}
}

fn count<T: Component>(app: &mut App) -> usize {
	app.world_mut().query_filtered::<(), With<T>>().iter(app.world()).count()
}

fn active_id(app: &App) -> Option<CharacterId> {
	app.world().get_resource::<ActiveCharacter>().map(|active| active.id)
}

fn delete_open(app: &App) -> bool {
	app.world().resource::<MenuModalStack>().contains(MenuModalId::CHARACTER_DELETE)
}

fn two_character_app() -> Result<(App, SaveRoot, CharacterId, CharacterId)> {
	let root = scratch_root();
	let jeff = CharacterId(11);
	let roamer = CharacterId(12);
	save_named(&root, jeff, "Jeff")?;
	save_named(&root, roamer, "Roamer")?;
	let app = gallery_app(root.clone(), jeff)?;
	Ok((app, root, jeff, roamer))
}

#[test]
fn gallery_replaces_live_buttons_with_prompt_chips() -> Result<()> {
	let (mut app, root, _, _) = two_character_app()?;
	assert_eq!(count::<GalleryPromptKind>(&mut app), 4);
	assert_eq!(count::<ScreenBack>(&mut app), 0);
	assert_eq!(count::<ScreenEdit>(&mut app), 0);
	let delete_rows = app
		.world_mut()
		.query::<&GalleryChoice>()
		.iter(app.world())
		.filter(|choice| **choice == GalleryChoice::Delete)
		.count();
	assert_eq!(delete_rows, 0);
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn pad_a_selects_the_highlighted_character() -> Result<()> {
	let (mut app, root, jeff, roamer) = two_character_app()?;
	assert_eq!(active_id(&app), Some(jeff));
	tap_pad(&mut app, GamepadButton::DPadDown)?;
	tap_pad(&mut app, GamepadButton::South)?;
	assert_eq!(active_id(&app), Some(roamer));
	assert!(count::<RequestShowCharacter>(&mut app) == 0);
	assert!(!delete_open(&app));
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn pad_b_leaves_the_gallery() -> Result<()> {
	let (mut app, root, _, _) = two_character_app()?;
	tap_pad(&mut app, GamepadButton::East)?;
	assert!(count::<RequestShowHome>(&mut app) >= 1);
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn pad_x_edits_the_highlighted_character() -> Result<()> {
	let (mut app, root, jeff, roamer) = two_character_app()?;
	tap_pad(&mut app, GamepadButton::DPadDown)?;
	tap_pad(&mut app, GamepadButton::West)?;
	let editing = app.world().get_resource::<EditingCharacter>().context("edit should open")?;
	assert_eq!(editing.id, roamer);
	assert!(count::<RequestShowCharacter>(&mut app) >= 1);
	assert_eq!(active_id(&app), Some(jeff));
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn pad_select_opens_delete_and_b_cancels_without_deleting() -> Result<()> {
	let (mut app, root, jeff, _) = two_character_app()?;
	tap_pad(&mut app, GamepadButton::Select)?;
	assert!(delete_open(&app));
	assert!(root.character_path(jeff).is_file());
	tap_pad(&mut app, GamepadButton::East)?;
	assert!(!delete_open(&app));
	assert!(root.character_path(jeff).is_file());
	assert_eq!(character_persist::load_active(&root), Some(jeff));
	assert_eq!(active_id(&app), Some(jeff));
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn pad_select_confirm_deletes_the_highlighted_character() -> Result<()> {
	let (mut app, root, jeff, roamer) = two_character_app()?;
	tap_pad(&mut app, GamepadButton::Select)?;
	assert!(delete_open(&app));
	tap_pad(&mut app, GamepadButton::DPadDown)?;
	tap_pad(&mut app, GamepadButton::South)?;
	assert!(!delete_open(&app));
	assert!(!root.character_path(jeff).is_file());
	assert!(root.character_path(roamer).is_file());
	assert_eq!(character_persist::load_active(&root), Some(roamer));
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn keyboard_bindings_match_pad_select_back_edit_and_delete() -> Result<()> {
	let (mut app, root, _jeff, roamer) = two_character_app()?;
	tap_key(&mut app, KeyCode::ArrowDown);
	tap_key(&mut app, KeyCode::Enter);
	assert_eq!(active_id(&app), Some(roamer));
	tap_key(&mut app, KeyCode::KeyE);
	let editing = app.world().get_resource::<EditingCharacter>().context("E edits")?;
	assert_eq!(editing.id, roamer);
	std::fs::remove_dir_all(&root.path)?;

	let (mut app, root, _, _) = two_character_app()?;
	tap_key(&mut app, KeyCode::Escape);
	assert!(count::<RequestShowHome>(&mut app) >= 1);
	std::fs::remove_dir_all(&root.path)?;

	let (mut app, root, jeff, _) = two_character_app()?;
	tap_key(&mut app, KeyCode::Delete);
	assert!(delete_open(&app));
	assert!(root.character_path(jeff).is_file());
	tap_key(&mut app, KeyCode::Escape);
	assert!(!delete_open(&app));
	assert!(root.character_path(jeff).is_file());
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}

#[test]
fn a_on_create_opens_create_and_select_does_not_delete() -> Result<()> {
	let (mut app, root, jeff, _) = two_character_app()?;
	let menu = app
		.world_mut()
		.query_filtered::<Entity, With<TextMenu>>()
		.iter(app.world())
		.next()
		.context("gallery menu")?;
	app.world_mut().get_mut::<TextMenu>(menu).context("text menu")?.selected = 0;
	app.update();
	tap_pad(&mut app, GamepadButton::Select)?;
	assert!(!delete_open(&app));
	assert!(root.character_path(jeff).is_file());
	tap_pad(&mut app, GamepadButton::South)?;
	assert!(count::<RequestShowCreateCharacter>(&mut app) >= 1);
	std::fs::remove_dir_all(&root.path)?;
	Ok(())
}
