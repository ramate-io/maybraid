use bevy::prelude::*;
use bevy::window::WindowFocused;
use camera_controls::look::{look_input_active, CameraLookEnabled};
use game_commands::command::TextEntryFocus;
use std::f32::consts::PI;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(mut commands: Commands) {
	let camera_pos = Vec3::new(-6.0, 3.2, 9.0);
	let look_at = Vec3::new(8.0, 1.2, 0.0);
	let transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();

	commands.spawn((
		Camera3d::default(),
		lod::LodViewer,
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.05, far: 4000.0, ..default() }),
		CameraController { speed: 8.0, sensitivity: 0.005, yaw, pitch },
	));
}

pub fn release_modifiers_on_focus_change(
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

fn command_held(keyboard: &ButtonInput<KeyCode>) -> bool {
	keyboard.any_pressed([
		KeyCode::SuperLeft,
		KeyCode::SuperRight,
		KeyCode::ControlLeft,
		KeyCode::ControlRight,
	])
}

pub fn camera_controller(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	if command_held(&keyboard_input) {
		mouse_motion.clear();
		return;
	}

	if look_input_active(look_enabled) {
		let mut mouse_delta = Vec2::ZERO;
		for event in mouse_motion.read() {
			mouse_delta += event.delta;
		}

		controller.yaw -= mouse_delta.x * controller.sensitivity;
		controller.pitch -= mouse_delta.y * controller.sensitivity;
		controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

		let yaw_quat = Quat::from_axis_angle(Vec3::Y, controller.yaw);
		let pitch_quat = Quat::from_axis_angle(Vec3::X, controller.pitch);
		transform.rotation = yaw_quat * pitch_quat;
	} else {
		mouse_motion.clear();
	}

	if text_focus.0 {
		return;
	}

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
		movement = movement.normalize() * controller.speed * time.delta_secs();
		transform.translation += movement;
	}
}
