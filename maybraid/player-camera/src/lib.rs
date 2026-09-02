//! Follow camera with POV / look-cone / FOV. Item users write [`PlayerCameraAim`].

mod follow;
mod look;

use bevy::prelude::*;
use bevy::window::WindowFocused;
use maybraid_character_controller::CharacterControlSystems;
use maybraid_player::{PlayerPoseSystems, PlayerSystems};

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
				PlayerCameraSystems::Aim.after(PlayerPoseSystems::Item),
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
	}
}

pub fn spawn_follow_camera(commands: &mut Commands) {
	use std::f32::consts::FRAC_PI_2;
	let yaw = -FRAC_PI_2;
	let pitch = -0.12;
	commands.spawn((
		Camera3d::default(),
		lod::LodViewer,
		Transform::from_translation(Vec3::new(-3.6, 1.8, 0.0)).looking_at(Vec3::Y * 0.65, Vec3::Y),
		Projection::Perspective(PerspectiveProjection {
			fov: follow::THIRD_PERSON_FOV,
			near: 0.05,
			far: 4000.0,
			..default()
		}),
		CameraController {
			sensitivity: 0.005,
			yaw,
			pitch,
			pov: CameraPov::ThirdPerson,
			focus: 0.0,
			focus_blend: 0.0,
		},
	));
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
