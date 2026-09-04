//! Cameras, clear color, preview look, and world pause — derived from [`GameFlow`].

use bevy::prelude::*;
use crozon_character_playground::CameraController as PreviewCameraController;
use maybraid_world::WorldGameplayEnabled;
use menu_components::MENU_CLEAR;
use menu_screens::{
	despawn_menu_screens, request_show_gallery, request_show_home, request_show_in_game,
	request_show_loading, InGameScreen, MenuScreen,
};

use crate::flow::{GameFlow, WorldPause};

/// Durham / vegetation sky wash while the 3D camera is live.
const WORLD_SKY: Color = Color::hsla(201.0, 0.69, 0.62, 1.0);

#[derive(Component)]
pub(crate) struct MenuUiCamera;

pub(crate) fn spawn_menu_ui_camera(mut commands: Commands) {
	commands.spawn((Camera2d, MenuUiCamera, Camera { order: 1, ..default() }));
}

pub(crate) fn enter_home(mut commands: Commands) {
	request_show_home(&mut commands);
}

pub(crate) fn enter_characters(mut commands: Commands) {
	request_show_gallery(&mut commands);
}

pub(crate) fn enter_loading(mut commands: Commands) {
	request_show_loading(&mut commands);
}

pub(crate) fn enter_world(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
	despawn_menu_screens(&mut commands, &screens);
}

pub(crate) fn enter_world_menu(mut commands: Commands) {
	request_show_in_game(&mut commands);
}

pub(crate) fn exit_world_menu(mut commands: Commands, overlay: Query<Entity, With<InGameScreen>>) {
	despawn_menu_screens(&mut commands, &overlay);
}

pub(crate) fn apply_shell_look(
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	mut clear: ResMut<ClearColor>,
	mut world_cameras: Query<&mut Camera, (With<Camera3d>, Without<MenuUiCamera>)>,
	mut ui_cameras: Query<&mut Camera, (With<MenuUiCamera>, Without<Camera3d>)>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
) {
	let flow = *flow.get();
	clear.0 = if flow == GameFlow::Home { MENU_CLEAR } else { WORLD_SKY };
	for mut camera in &mut world_cameras {
		camera.is_active = true;
	}
	for mut camera in &mut ui_cameras {
		camera.is_active = false;
	}
	gameplay.0 =
		flow == GameFlow::World && pause.is_some_and(|pause| *pause.get() == WorldPause::Playing);
}

/// Temporary: vegetation assumes a unique `Camera3d`, so preview look is bolted
/// onto the world camera instead of spawning a second one.
pub(crate) fn attach_preview_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<PreviewCameraController>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(PreviewCameraController {
			speed: 6.0,
			sensitivity: 0.005,
			yaw: 0.0,
			pitch: 0.0,
		});
	}
}

pub(crate) fn detach_preview_camera(
	mut commands: Commands,
	preview: Query<Entity, (With<Camera3d>, With<PreviewCameraController>)>,
) {
	for entity in &preview {
		commands.entity(entity).remove::<PreviewCameraController>();
	}
}
