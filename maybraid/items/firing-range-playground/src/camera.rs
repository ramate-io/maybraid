//! Third-person orbit. Yaw/pitch come from [`crate::control::apply_intents`].

use bevy::prelude::*;
use bevy::window::WindowFocused;
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct CameraController {
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub(crate) fn setup_camera(mut commands: Commands) {
	// Behind the player, looking downrange (+X). Follow overwrites translation.
	let yaw = -FRAC_PI_2;
	let pitch = -0.12;
	commands.spawn((
		Camera3d::default(),
		lod::LodViewer,
		Transform::from_translation(Vec3::new(-3.6, 1.8, 0.0)).looking_at(Vec3::Y * 0.65, Vec3::Y),
		Projection::Perspective(PerspectiveProjection { near: 0.05, far: 4000.0, ..default() }),
		CameraController { sensitivity: 0.005, yaw, pitch },
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
