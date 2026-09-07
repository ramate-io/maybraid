//! Cameras, clear color, preview look, and world pause — derived from [`GameFlow`].
//!
//! Home / Characters use the world `Camera3d` as [`IsDefaultUiCamera`] with
//! preview-only [`RenderLayers`], matching [PR #729](https://github.com/ramate-io/maybraid/pull/729).
//! Isolation is camera layers, not `Visibility` on LOD / trimesh hosts.
//!
//! Async load-in follows [`efa73ad`](https://github.com/ramate-io/maybraid/commit/efa73adf):
//! a `Camera2d` exists only during [`GameFlow::LoadingWorld`]. A persistent
//! second camera would steal UI. Terrain streaming stays off on menu shells.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use crozon_character_playground::CameraController as PreviewCameraController;
use maybraid_world::{
	PlayerPhysicsEnabled, TerrainStreamingEnabled, WorldGameplayEnabled, WorldSceneryVisible,
};
use menu_components::MENU_CLEAR;
use menu_playground::{CharacterPreviewLight, CharacterPreviewRoot};
use menu_screens::{
	despawn_menu_screens, request_show_gallery, request_show_home, request_show_in_game,
	request_show_loading, MenuScreen,
};

use crate::flow::{GameFlow, WorldPause};

/// Durham / vegetation sky wash while the 3D camera is live.
const WORLD_SKY: Color = Color::hsla(201.0, 0.69, 0.62, 1.0);
const PREVIEW_EYE: Vec3 = Vec3::new(0.0, 1.6, 3.5);
const PREVIEW_LOOK: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const WORLD_RENDER_LAYER: usize = 0;
const PREVIEW_RENDER_LAYER: usize = 1;

#[derive(Component)]
pub(crate) struct LoadingBackdropCamera;

pub(crate) fn spawn_loading_backdrop(mut commands: Commands) {
	commands.spawn((
		Camera2d,
		LoadingBackdropCamera,
		IsDefaultUiCamera,
		Camera { order: 1, clear_color: ClearColorConfig::Custom(MENU_CLEAR), ..default() },
	));
}

pub(crate) fn despawn_loading_backdrop(
	mut commands: Commands,
	cameras: Query<Entity, With<LoadingBackdropCamera>>,
) {
	for entity in &cameras {
		commands.entity(entity).despawn();
	}
}

pub(crate) fn enter_home(mut commands: Commands) {
	request_show_home(&mut commands);
}

pub(crate) fn enter_characters(mut commands: Commands) {
	request_show_gallery(&mut commands);
}

pub(crate) fn enter_loading_world(
	mut commands: Commands,
	screens: Query<Entity, With<MenuScreen>>,
) {
	despawn_menu_screens(&mut commands, screens);
	request_show_loading(&mut commands);
}

pub(crate) fn enter_world(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
	despawn_menu_screens(&mut commands, screens);
}

pub(crate) fn enter_world_menu(mut commands: Commands) {
	request_show_in_game(&mut commands);
}

pub(crate) fn exit_world_menu(mut commands: Commands, overlay: Query<Entity, With<MenuScreen>>) {
	despawn_menu_screens(&mut commands, overlay);
}

pub(crate) fn apply_shell_look(
	mut commands: Commands,
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	mut clear: ResMut<ClearColor>,
	mut world_cameras: Query<
		(Entity, &mut Camera),
		(With<Camera3d>, Without<LoadingBackdropCamera>),
	>,
	mut loading_cameras: Query<&mut Camera, (With<LoadingBackdropCamera>, Without<Camera3d>)>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut physics: ResMut<PlayerPhysicsEnabled>,
	mut streaming: ResMut<TerrainStreamingEnabled>,
	mut scenery: ResMut<WorldSceneryVisible>,
) {
	let flow = *flow.get();
	let loading = flow == GameFlow::LoadingWorld;
	let menu = matches!(flow, GameFlow::Home | GameFlow::Characters | GameFlow::LoadingWorld);
	clear.0 = if menu { MENU_CLEAR } else { WORLD_SKY };
	let layers = camera_render_layers(flow);
	for (entity, mut camera) in &mut world_cameras {
		camera.is_active = !loading;
		commands.entity(entity).insert(layers.clone());
		if loading {
			commands.entity(entity).remove::<IsDefaultUiCamera>();
		} else {
			commands.entity(entity).insert(IsDefaultUiCamera);
		}
	}
	for mut camera in &mut loading_cameras {
		camera.is_active = loading;
	}
	streaming.0 = matches!(flow, GameFlow::LoadingWorld | GameFlow::World);
	scenery.0 = flow == GameFlow::World;
	let playing =
		flow == GameFlow::World && pause.is_some_and(|pause| *pause.get() == WorldPause::Playing);
	gameplay.0 = playing;
	physics.0 = playing;
}

fn camera_render_layers(flow: GameFlow) -> RenderLayers {
	match flow {
		GameFlow::Home | GameFlow::Characters => RenderLayers::layer(PREVIEW_RENDER_LAYER),
		GameFlow::LoadingWorld | GameFlow::World => RenderLayers::layer(WORLD_RENDER_LAYER),
	}
}

/// Match the eye used by the standalone menu playground.
pub(crate) fn attach_preview_camera(
	mut commands: Commands,
	mut cameras: Query<(Entity, &mut Transform), With<Camera3d>>,
) {
	for (entity, mut transform) in &mut cameras {
		*transform = Transform::from_translation(PREVIEW_EYE).looking_at(PREVIEW_LOOK, Vec3::Y);
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

/// Keep character previews and their lights isolated from the streamed world.
pub(crate) fn stamp_preview_render_layers(
	preview_roots: Query<Entity, With<CharacterPreviewRoot>>,
	preview_lights: Query<Entity, With<CharacterPreviewLight>>,
	children: Query<&Children>,
	layered: Query<(), With<RenderLayers>>,
	mut commands: Commands,
) {
	let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
	for root in preview_roots.iter().chain(preview_lights.iter()) {
		for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
			if !layered.contains(entity) {
				commands.entity(entity).insert(preview.clone());
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::camera::visibility::RenderLayers;

	use super::{PREVIEW_RENDER_LAYER, WORLD_RENDER_LAYER, camera_render_layers};
	use crate::flow::GameFlow;

	#[test]
	fn menu_camera_sees_preview_only() {
		let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
		let world = RenderLayers::layer(WORLD_RENDER_LAYER);
		for flow in [GameFlow::Home, GameFlow::Characters] {
			assert!(camera_render_layers(flow).intersects(&preview));
			assert!(!camera_render_layers(flow).intersects(&world));
		}
	}

	#[test]
	fn loading_and_world_cameras_see_default_layer() {
		let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
		let world = RenderLayers::layer(WORLD_RENDER_LAYER);
		for flow in [GameFlow::LoadingWorld, GameFlow::World] {
			assert!(camera_render_layers(flow).intersects(&world));
			assert!(!camera_render_layers(flow).intersects(&preview));
		}
	}
}
