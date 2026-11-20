use std::f32::consts::PI;

use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use camera_controls::look::{look_input_active, CameraLookEnabled};
use game_commands::command::TextEntryFocus;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(mut commands: Commands) {
	let transform =
		Transform::from_xyz(10.0, 3.2, 16.0).looking_at(Vec3::new(8.0, 0.3, 8.0), Vec3::Y);
	let (yaw, pitch) = yaw_pitch(transform.rotation);
	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection {
			near: 0.1,
			far: 400.0,
			fov: 55.0_f32.to_radians(),
			..default()
		}),
		Msaa::Off,
		DepthPrepass,
		CameraController { speed: 8.0, sensitivity: 0.003, yaw, pitch },
	));
}

pub fn yaw_pitch(rotation: Quat) -> (f32, f32) {
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	(yaw, sin_pitch.asin())
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

	if look_input_active(look_enabled) {
		let mut mouse_delta = Vec2::ZERO;
		for event in mouse_motion.read() {
			mouse_delta += event.delta;
		}
		controller.yaw -= mouse_delta.x * controller.sensitivity;
		controller.pitch -= mouse_delta.y * controller.sensitivity;
		controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
		transform.rotation = Quat::from_axis_angle(Vec3::Y, controller.yaw)
			* Quat::from_axis_angle(Vec3::X, controller.pitch);
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
	if keyboard_input.pressed(KeyCode::ShiftLeft) {
		movement -= Vec3::Y;
	}
	if movement.length_squared() > 0.0 {
		transform.translation += movement.normalize() * controller.speed * time.delta_secs();
	}
}
