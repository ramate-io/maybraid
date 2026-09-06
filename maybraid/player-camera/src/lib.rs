//! Follow camera with POV / look-cone / FOV. Item users write [`PlayerCameraAim`].

mod follow;
mod look;

use bevy::prelude::*;
use bevy::window::WindowFocused;
use maybraid_character_controller::CharacterControlSystems;
use player::{PlayerPlugin, PlayerPoseSystems, PlayerSystems};
use std::f32::consts::FRAC_PI_2;

pub use follow::{sync_camera_fov, sync_first_person_head_visibility};
pub use look::{CameraController, CameraPov};

/// Camera schedule. Item crates add aim writers to [`PlayerCameraSystems::Aim`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerCameraSystems {
	Look,
	Body,
	Aim,
	Follow,
	Apply,
}

/// Per-camera follow / FOV / look-cone knobs. Runtime look state lives on [`CameraController`].
#[derive(Component, Clone, Copy, Debug)]
pub struct FollowCamera {
	pub distance: f32,
	pub height: f32,
	pub look_height: f32,
	pub shoulder_offset: f32,
	pub eye_forward: f32,
	pub focus_blend_speed: f32,
	pub third_person_fov: f32,
	pub first_person_fov: f32,
	pub sight_fov: f32,
	pub max_look_yaw: f32,
	pub body_turn_rate: f32,
	pub sensitivity: f32,
	pub near: f32,
	pub far: f32,
}

impl Default for FollowCamera {
	fn default() -> Self {
		Self {
			distance: 3.6,
			height: 1.1,
			look_height: 0.65,
			shoulder_offset: 0.7,
			eye_forward: 0.04,
			focus_blend_speed: 12.0,
			third_person_fov: 45.0_f32.to_radians(),
			first_person_fov: 75.0_f32.to_radians(),
			sight_fov: 50.0_f32.to_radians(),
			max_look_yaw: 15.0_f32.to_radians(),
			body_turn_rate: 8.0,
			sensitivity: 0.005,
			near: 0.05,
			far: 4000.0,
		}
	}
}

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				PlayerCameraSystems::Look
					.after(CharacterControlSystems)
					.before(PlayerSystems::Intent),
				PlayerCameraSystems::Body
					.after(PlayerCameraSystems::Look)
					.before(PlayerPoseSystems::Item),
				PlayerCameraSystems::Aim.after(PlayerPoseSystems::Overlay),
				PlayerCameraSystems::Follow.after(PlayerCameraSystems::Aim),
				PlayerCameraSystems::Apply.after(PlayerCameraSystems::Follow),
			),
		)
		.add_systems(
			Update,
			(look::apply_look_intents, look::sync_player_look)
				.chain()
				.in_set(PlayerCameraSystems::Look),
		)
		.add_systems(
			Update,
			(look::sync_yaw_owner, look::turn_body_with_look)
				.chain()
				.in_set(PlayerCameraSystems::Body),
		)
		.add_systems(Update, follow::follow_character_camera.in_set(PlayerCameraSystems::Follow))
		.add_systems(
			Update,
			(follow::sync_camera_fov, follow::sync_first_person_head_visibility)
				.in_set(PlayerCameraSystems::Apply),
		)
		.add_systems(Update, release_modifiers_on_focus_change);
		if app.is_plugin_added::<PlayerPlugin>() {
			app.configure_sets(Update, PlayerCameraSystems::Body.before(PlayerSystems::Locomotion));
		}
	}
}

pub fn spawn_follow_camera(commands: &mut Commands) -> Entity {
	let follow = FollowCamera::default();
	let yaw = -FRAC_PI_2;
	let pitch = -0.12;
	commands
		.spawn((
			Camera3d::default(),
			lod::LodViewer,
			follow,
			Transform::from_translation(Vec3::new(
				-follow.distance,
				follow.height + follow.look_height,
				0.0,
			))
			.looking_at(Vec3::Y * follow.look_height, Vec3::Y),
			Projection::Perspective(PerspectiveProjection {
				fov: follow.third_person_fov,
				near: follow.near,
				far: follow.far,
				..default()
			}),
			CameraController {
				yaw,
				pitch,
				pov: CameraPov::ThirdPerson,
				focus: 0.0,
				focus_blend: 0.0,
			},
		))
		.id()
}

fn release_modifiers_on_focus_change(
	mut keyboard: ResMut<ButtonInput<KeyCode>>,
	mut focus: MessageReader<WindowFocused>,
) {
	if focus.read().next().is_none() {
		return;
	}
	for key in [
		KeyCode::ShiftLeft,
		KeyCode::ShiftRight,
		KeyCode::SuperLeft,
		KeyCode::SuperRight,
		KeyCode::ControlLeft,
		KeyCode::ControlRight,
		KeyCode::AltLeft,
		KeyCode::AltRight,
	] {
		keyboard.release(key);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crozon_characters::CharacterMotionSystems;
	use maybraid_character_controller::CharacterIntent;

	#[test]
	fn presentation_only_camera_allows_world_body_after_animation() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_message::<WindowFocused>()
			.add_plugins(player::PlayerPresentationPlugin)
			.add_plugins(PlayerCameraPlugin)
			.configure_sets(
				Update,
				(
					CharacterMotionSystems::Elevation.after(CharacterMotionSystems::Anim),
					PlayerCameraSystems::Body
						.after(CharacterMotionSystems::Anim)
						.before(CharacterMotionSystems::Elevation),
				),
			);
		app.update();
	}
}
