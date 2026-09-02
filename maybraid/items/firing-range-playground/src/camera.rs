//! Third-person orbit and first-person look. Yaw/pitch come from intents.

use bevy::prelude::*;
use bevy::window::WindowFocused;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::character::PlayerVisual;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CameraPov {
	#[default]
	ThirdPerson,
	FirstPerson,
}

impl CameraPov {
	pub(crate) fn toggle(&mut self) {
		*self = match self {
			Self::ThirdPerson => Self::FirstPerson,
			Self::FirstPerson => Self::ThirdPerson,
		};
	}
}

#[derive(Component)]
pub struct CameraController {
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
	pub(crate) pov: CameraPov,
	pub(crate) focus: f32,
	pub(crate) focus_blend: f32,
}

/// Free look relative to the body before the torso has to follow.
const MAX_LOOK_YAW: f32 = 60.0_f32.to_radians();
const BODY_TURN_RATE: f32 = 8.0;

pub(crate) fn setup_camera(mut commands: Commands) {
	// Behind the player, looking downrange (+X). Follow overwrites translation.
	let yaw = -FRAC_PI_2;
	let pitch = -0.12;
	commands.spawn((
		Camera3d::default(),
		lod::LodViewer,
		Transform::from_translation(Vec3::new(-3.6, 1.8, 0.0)).looking_at(Vec3::Y * 0.65, Vec3::Y),
		Projection::Perspective(PerspectiveProjection { near: 0.05, far: 4000.0, ..default() }),
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

pub(crate) fn release_modifiers_on_focus_change(
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

/// When first-person look leaves the body cone, turn the torso to catch up.
pub(crate) fn turn_body_with_look(
	time: Res<Time>,
	mut cameras: Query<&mut CameraController, With<Camera3d>>,
	mut visuals: Query<&mut Transform, (With<PlayerVisual>, Without<Camera3d>)>,
) {
	let Ok(mut controller) = cameras.single_mut() else {
		return;
	};
	if controller.pov != CameraPov::FirstPerson {
		return;
	}
	let Ok(mut visual) = visuals.single_mut() else {
		return;
	};

	let body = body_yaw(&visual);
	let target = follow_body_yaw(controller.yaw, body, MAX_LOOK_YAW);
	let step = wrap_to_pi(target - body);
	let max_step = BODY_TURN_RATE * time.delta_secs();
	let applied = step.abs().min(max_step).copysign(step);
	if applied.abs() > 1e-5 {
		set_body_yaw(&mut visual, body + applied);
	}
	controller.yaw = clamp_look_yaw(controller.yaw, body_yaw(&visual), MAX_LOOK_YAW);
}

/// Camera yaw of mesh +Z (`-forward()`), matching `face_player` / gun facing.
fn body_yaw(visual: &Transform) -> f32 {
	camera_yaw_of_forward(-*visual.forward())
}

fn set_body_yaw(visual: &mut Transform, yaw: f32) {
	let forward = Quat::from_axis_angle(Vec3::Y, yaw) * -Vec3::Z;
	visual.look_to(-forward, Vec3::Y);
}

fn camera_yaw_of_forward(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		(-n.x).atan2(-n.z)
	}
}

fn wrap_to_pi(angle: f32) -> f32 {
	(angle + PI).rem_euclid(2.0 * PI) - PI
}

fn follow_body_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	look_yaw - delta.clamp(-max_delta, max_delta)
}

fn clamp_look_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	body_yaw + delta.clamp(-max_delta, max_delta)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pov_toggle_round_trips() {
		let mut pov = CameraPov::ThirdPerson;
		pov.toggle();
		assert_eq!(pov, CameraPov::FirstPerson);
		pov.toggle();
		assert_eq!(pov, CameraPov::ThirdPerson);
	}

	#[test]
	fn body_yaw_matches_camera_yaw_when_mesh_faces_downrange() {
		let mut visual = Transform::IDENTITY;
		visual.look_to(-Vec3::X, Vec3::Y);
		assert!((body_yaw(&visual) + FRAC_PI_2).abs() < 1e-4, "{}", body_yaw(&visual));
		let mut round_trip = Transform::IDENTITY;
		set_body_yaw(&mut round_trip, -FRAC_PI_2);
		assert!((body_yaw(&round_trip) + FRAC_PI_2).abs() < 1e-4, "{}", body_yaw(&round_trip));
	}

	#[test]
	fn body_stays_put_inside_look_cone() {
		let look = -FRAC_PI_2 + 0.3;
		let body = -FRAC_PI_2;
		assert!((follow_body_yaw(look, body, MAX_LOOK_YAW) - body).abs() < 1e-5);
	}

	#[test]
	fn body_follows_when_look_exceeds_cone() {
		let look = -FRAC_PI_2 + MAX_LOOK_YAW + 0.4;
		let body = -FRAC_PI_2;
		let target = follow_body_yaw(look, body, MAX_LOOK_YAW);
		assert!(target < look);
		assert!((clamp_look_yaw(look, target, MAX_LOOK_YAW) - look).abs() < 1e-4);
	}
}
