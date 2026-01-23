use bevy::{core_pipeline::prepass::DepthPrepass, prelude::*};
use std::f32::consts::PI;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
	pub character_mode: bool,
	pub velocity: Vec3, // For gravity and movement in character mode
}

pub fn setup_camera(mut commands: Commands) {
	let camera_pos = Vec3::new(20.0, 20.0, 20.0);
	let look_at = Vec3::new(0.0, 0.0, 0.0);

	log::info!("Setting up camera at position: {:?}, looking at: {:?}", camera_pos, look_at);

	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(camera_pos.x, camera_pos.y, camera_pos.z).looking_at(look_at, Vec3::Y),
		Projection::Perspective(PerspectiveProjection {
			near: 0.0001, // 10 cm
			far: 2.0,     // 2000 km
			..default()
		}),
		CameraController {
			speed: 20.0,
			sensitivity: 0.005,
			yaw: -90.0_f32.to_radians(),
			pitch: -20.0_f32.to_radians(),
			character_mode: false,
			velocity: Vec3::ZERO,
		},
		DepthPrepass,
	));
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

	// Toggle character mode with 'C' key
	if keyboard_input.just_pressed(KeyCode::KeyC) {
		controller.character_mode = !controller.character_mode;
		if controller.character_mode {
			log::info!("Character mode enabled");
			// When entering character mode, drop to terrain
			controller.velocity = Vec3::ZERO;
		} else {
			log::info!("Character mode disabled");
			controller.velocity = Vec3::ZERO;
		}
	}

	// Handle mouse look
	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}

	controller.yaw -= mouse_delta.x * controller.sensitivity;
	controller.pitch -= mouse_delta.y * controller.sensitivity;
	controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

	// Update camera rotation
	let yaw_quat = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch_quat = Quat::from_axis_angle(Vec3::X, controller.pitch);
	transform.rotation = yaw_quat * pitch_quat;

	// Free-fly mode: normal movement
	free_fly_movement(&keyboard_input, &time, &mut transform, &mut controller);
}

fn free_fly_movement(
	keyboard_input: &Res<ButtonInput<KeyCode>>,
	time: &Res<Time>,
	transform: &mut Transform,
	controller: &mut CameraController,
) {
	// Handle movement
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

	if movement.length() > 0.0 {
		movement = movement.normalize() * controller.speed * time.delta_secs();
		transform.translation += movement;
	}
}
