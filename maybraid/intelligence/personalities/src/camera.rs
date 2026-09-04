//! Free-look survey camera over the 400 m pad.

use bevy::prelude::*;
use std::f32::consts::PI;

const CAMERA_HEIGHT: f32 = 220.0;
const CAMERA_BACK: f32 = 40.0;
const FLY_SPEED: f32 = 140.0;
const MAX_LOOK_DELTA: f32 = 80.0;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(mut commands: Commands) {
	let look_at = Vec3::ZERO;
	let camera_pos = look_at + Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_BACK);
	let transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.3, far: 4_000.0, ..default() }),
		CameraController { speed: FLY_SPEED, sensitivity: 0.005, yaw, pitch },
	));
}

pub fn release_modifiers_on_focus_change(
	mut keyboard: ResMut<ButtonInput<KeyCode>>,
	mut focus: MessageReader<bevy::window::WindowFocused>,
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

pub fn camera_controller(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}
	if mouse_delta.length() <= MAX_LOOK_DELTA {
		controller.yaw -= mouse_delta.x * controller.sensitivity;
		controller.pitch -= mouse_delta.y * controller.sensitivity;
		controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
	}

	let yaw_quat = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch_quat = Quat::from_axis_angle(Vec3::X, controller.pitch);
	transform.rotation = yaw_quat * pitch_quat;

	let mut movement = Vec3::ZERO;
	let forward = transform.forward();
	let right = transform.right();
	if keyboard_input.pressed(KeyCode::KeyW) {
		movement += *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyS) {
		movement -= *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyA) {
		movement -= *right;
	}
	if keyboard_input.pressed(KeyCode::KeyD) {
		movement += *right;
	}
	if keyboard_input.pressed(KeyCode::Space) {
		movement += Vec3::Y;
	}
	if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
		movement -= Vec3::Y;
	}

	if movement.length_squared() > 0.0 {
		let sprint = if keyboard_input.pressed(KeyCode::ControlLeft)
			|| keyboard_input.pressed(KeyCode::ControlRight)
		{
			2.4
		} else {
			1.0
		};
		movement = movement.normalize() * controller.speed * sprint * time.delta_secs();
		transform.translation += movement;
	}
}

pub fn ground_look_at(transform: &Transform) -> Option<Vec3> {
	let origin = transform.translation;
	let dir = *transform.forward();
	if dir.y.abs() < 1e-4 {
		return None;
	}
	let t = -origin.y / dir.y;
	if t <= 0.0 {
		return None;
	}
	Some(origin + dir * t)
}

fn yaw_pitch_from_rotation(rotation: Quat) -> (f32, f32) {
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = (2.0 * (w * x - y * z)).clamp(-1.0, 1.0);
	let pitch = sin_pitch.asin();
	(yaw, pitch)
}
