//! Cameras, clear color, preview look, and world pause — derived from [`GameFlow`].
//!
//! Home / Characters use the world `Camera3d` with preview-only
//! [`RenderLayers`]. World scenery remains on the default layer so menu
//! transitions do not churn LOD hosts or physics colliders.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use crozon_character_playground::CameraController as PreviewCameraController;
use maybraid_world::WorldGameplayEnabled;
use menu_components::MENU_CLEAR;
use menu_playground::{CharacterPreviewLight, CharacterPreviewRoot};
use menu_screens::{
	despawn_menu_screens, request_show_gallery, request_show_home, request_show_in_game,
	InGameScreen, MenuScreen,
};

use crate::flow::{GameFlow, WorldPause};

/// Durham / vegetation sky wash while the 3D camera is live.
const WORLD_SKY: Color = Color::hsla(201.0, 0.69, 0.62, 1.0);
const PREVIEW_EYE: Vec3 = Vec3::new(0.0, 1.6, 3.5);
const PREVIEW_LOOK: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const WORLD_RENDER_LAYER: usize = 0;
const PREVIEW_RENDER_LAYER: usize = 1;

pub(crate) fn enter_home(mut commands: Commands) {
	request_show_home(&mut commands);
}

pub(crate) fn enter_characters(mut commands: Commands) {
	request_show_gallery(&mut commands);
}

pub(crate) fn enter_world(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
	despawn_menu_screens(&mut commands, screens);
}

pub(crate) fn enter_world_menu(mut commands: Commands) {
	request_show_in_game(&mut commands);
}

pub(crate) fn exit_world_menu(mut commands: Commands, overlay: Query<Entity, With<InGameScreen>>) {
	despawn_menu_screens(&mut commands, overlay);
}

pub(crate) fn apply_shell_look(
	mut commands: Commands,
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	mut clear: ResMut<ClearColor>,
	mut world_cameras: Query<(Entity, &mut Camera), With<Camera3d>>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
) {
	let flow = *flow.get();
	let menu = matches!(flow, GameFlow::Home | GameFlow::Characters);
	clear.0 = if menu { MENU_CLEAR } else { WORLD_SKY };
	let layers = camera_render_layers(flow);
	for (entity, mut camera) in &mut world_cameras {
		camera.is_active = true;
		commands.entity(entity).insert((layers.clone(), IsDefaultUiCamera));
	}
	gameplay.0 =
		flow == GameFlow::World && pause.is_some_and(|pause| *pause.get() == WorldPause::Playing);
}

fn camera_render_layers(flow: GameFlow) -> RenderLayers {
	match flow {
		GameFlow::Home | GameFlow::Characters => RenderLayers::layer(PREVIEW_RENDER_LAYER),
		GameFlow::World => RenderLayers::layer(WORLD_RENDER_LAYER),
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

	use super::{camera_render_layers, PREVIEW_RENDER_LAYER, WORLD_RENDER_LAYER};
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
	fn world_camera_sees_default_layer_only() {
		let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
		let world = RenderLayers::layer(WORLD_RENDER_LAYER);
		assert!(camera_render_layers(GameFlow::World).intersects(&world));
		assert!(!camera_render_layers(GameFlow::World).intersects(&preview));
	}
}
