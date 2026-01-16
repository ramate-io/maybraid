use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;
use skill_map::viewport::{ApplyCameraTransform, SkillMapViewport, SkillMapViewportId};

use std::f32::consts::PI;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
	pub lock_skillmap_movement_time_remaining: f32,
}

#[derive(Component)]
pub struct LockSkillmapMovement {
	pub time_remaining: f32,
}

pub fn setup_camera(mut commands: Commands) {
	// Position camera to look at the origin (0, 0, 0) where the tree is
	// Tree is about 0.005km (5m) tall, so position camera at a good viewing distance
	let camera_pos = Vec3::new(0.0, 10.0, 20.0); // 10m up, 20m back
	let look_at = Vec3::ZERO; // Look at origin

	log::info!("Setting up camera at position: {:?}, looking at: {:?}", camera_pos, look_at);

	// Create transform that looks at origin
	let transform =
		Transform::from_xyz(camera_pos.x, camera_pos.y, camera_pos.z).looking_at(look_at, Vec3::Y);

	// Extract yaw and pitch from the transform's rotation quaternion
	// We'll extract Euler angles from the quaternion
	let rotation = transform.rotation;

	// Extract Euler angles (ZYX order: yaw around Y, pitch around X, roll around Z)
	// Bevy uses ZYX Euler order by default
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);

	// Calculate yaw (rotation around Y axis)
	// yaw = atan2(2*(w*y + x*z), 1 - 2*(y*y + z*z))
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);

	// Calculate pitch (rotation around X axis)
	// pitch = asin(2*(w*x - y*z))
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();

	log::info!(
		"Camera rotation: {:?}, yaw: {}°, pitch: {}°",
		rotation,
		yaw.to_degrees(),
		pitch.to_degrees()
	);

	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection {
			near: 0.1,   // 10 cm
			far: 2000.0, // 2 m
			..default()
		}),
		CameraController {
			speed: 10.0, // 1m/s
			sensitivity: 0.005,
			yaw,
			pitch,
			lock_skillmap_movement_time_remaining: 0.0,
		},
	));
}

/// Apply deadzone to analog stick input to prevent drift
fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
	if value.abs() < deadzone {
		0.0
	} else {
		// Scale so output starts from zero after deadzone
		let sign = value.signum();
		(value - sign * deadzone) / (1.0 - deadzone)
	}
}

/// Get gamepad movement input from left stick
fn get_gamepad_movement(gamepad: &Gamepad, deadzone: f32) -> Vec2 {
	let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
	let left_stick_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);

	Vec2::new(
		apply_deadzone(left_stick_x, deadzone),
		apply_deadzone(left_stick_y, deadzone), // Invert Y for standard gamepad behavior
	)
}

/// Get gamepad camera look input from right stick
fn get_gamepad_look(gamepad: &Gamepad, deadzone: f32) -> Vec2 {
	let right_stick_x = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
	let right_stick_y = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);

	Vec2::new(
		apply_deadzone(right_stick_x, deadzone),
		apply_deadzone(-right_stick_y, deadzone), // Invert Y for standard camera look
	)
}

/// Get vertical movement from gamepad triggers or buttons
/// Returns positive for up, negative for down
fn get_gamepad_vertical_movement(gamepad: &Gamepad) -> f32 {
	let mut vertical = 0.0;

	if gamepad.pressed(GamepadButton::South) {
		log::info!("South button pressed");
		vertical += 1.0;
	}

	vertical
}

pub fn camera_controller(
	mut commands: Commands,
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	gamepad_query: Query<(&Name, &Gamepad)>,
	time: Res<Time>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
	skillmap_viewport_query: Query<Entity, With<SkillMapViewport>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	controller.lock_skillmap_movement_time_remaining -= time.delta_secs();
	if controller.lock_skillmap_movement_time_remaining < 0.0 {
		controller.lock_skillmap_movement_time_remaining = 0.0;
	}

	const GAMEPAD_DEADZONE: f32 = 0.15;
	const GAMEPAD_LOOK_SENSITIVITY: f32 = 3.0; // Multiplier for gamepad look sensitivity

	// Handle mouse look
	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}

	// Handle gamepad look (right stick)

	if let Ok((_name, gamepad)) = gamepad_query.single() {
		let gamepad_look = get_gamepad_look(gamepad, GAMEPAD_DEADZONE);
		if gamepad_look.length() > 0.0 {
			// Gamepad look uses delta time for smooth movement
			controller.yaw -= gamepad_look.x
				* controller.sensitivity
				* GAMEPAD_LOOK_SENSITIVITY
				* time.delta_secs()
				* 60.0;
			controller.pitch -= gamepad_look.y
				* controller.sensitivity
				* GAMEPAD_LOOK_SENSITIVITY
				* time.delta_secs()
				* 60.0;
		}
	}

	// Apply mouse look (if any)
	controller.yaw -= mouse_delta.x * controller.sensitivity;
	controller.pitch -= mouse_delta.y * controller.sensitivity;
	controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

	// Update camera rotation
	let yaw_quat = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch_quat = Quat::from_axis_angle(Vec3::X, controller.pitch);
	transform.rotation = yaw_quat * pitch_quat;

	// Free-fly movement
	let mut movement = Vec3::ZERO;
	let forward = transform.forward();
	let right = transform.right();

	// Keyboard movement
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

	let mut gamepad_movement = Vec2::ZERO;
	let mut left_bumper_pressed = false;
	if let Ok((_name, gamepad)) = gamepad_query.single() {
		// Gamepad movement (left stick)
		gamepad_movement = get_gamepad_movement(gamepad, GAMEPAD_DEADZONE);
		if gamepad_movement.length() > 0.0 {
			// Apply movement relative to camera orientation
			movement += *forward * gamepad_movement.y * 5.0;
			movement += *right * gamepad_movement.x * 5.0;
		}

		// Gamepad vertical movement (triggers)
		let vertical = get_gamepad_vertical_movement(gamepad);
		if vertical.abs() > 0.01 {
			movement += Vec3::Y * vertical;
		}

		if gamepad.pressed(GamepadButton::LeftTrigger) {
			left_bumper_pressed = true;
		}
	}

	if movement.length() > 0.0 {
		movement = movement.normalize() * controller.speed * time.delta_secs();
		transform.translation += movement;
	}

	if controller.lock_skillmap_movement_time_remaining <= 0.0
		&& (keyboard_input.pressed(KeyCode::ShiftLeft)
			|| keyboard_input.pressed(KeyCode::ShiftRight)
			|| left_bumper_pressed)
	{
		// mark the skillmap viewport with a burnt orange border color
		if let Ok(skillmap_viewport) = skillmap_viewport_query.single() {
			commands
				.entity(skillmap_viewport)
				.insert(BorderColor::all(Color::srgb(1.0, 0.5, 0.0)));
		}

		let mut movement_2d = Vec3::ZERO;
		movement_2d.x += gamepad_movement.x;
		movement_2d.y += gamepad_movement.y;
		let mut movement_flag = false;

		if gamepad_movement.length() > 0.0 {
			movement_flag = true;
		}

		if keyboard_input.pressed(KeyCode::KeyW) {
			movement_2d.y += 2.0;
			movement_flag = true;
		}
		if keyboard_input.pressed(KeyCode::KeyS) {
			movement_2d.y -= 2.0;
			movement_flag = true;
		}
		if keyboard_input.pressed(KeyCode::KeyA) {
			movement_2d.x -= 2.0;
			movement_flag = true;
		}
		if keyboard_input.pressed(KeyCode::KeyD) {
			movement_2d.x += 2.0;
			movement_flag = true;
		}

		if movement_flag {
			let transform_2d = Transform::from_translation(movement_2d);
			commands.spawn((SkillMapViewportId(0), ApplyCameraTransform::Change2d, transform_2d));
		}
	} else {
		// unmark the skillmap viewport with a white border color
		if let Ok(skillmap_viewport) = skillmap_viewport_query.single() {
			commands.entity(skillmap_viewport).insert(BorderColor::all(Color::WHITE));
		}

		commands.spawn((
			SkillMapViewportId(0),
			ApplyCameraTransform::Value,
			Transform::from_translation(Vec3::new(0.0, 0.0, 1.000)),
		));
	}
}

pub fn lock_skillmap_movement(
	mut commands: Commands,
	query: Query<(Entity, &LockSkillmapMovement), Added<LockSkillmapMovement>>,
	mut camera_query: Query<&mut CameraController, With<Camera3d>>,
) {
	let Ok(mut controller) = camera_query.single_mut() else {
		return;
	};

	for (entity, lock_skillmap_movement) in query.iter() {
		controller.lock_skillmap_movement_time_remaining = lock_skillmap_movement.time_remaining;
		commands.entity(entity).despawn();
	}
}
