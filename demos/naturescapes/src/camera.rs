use crate::terrain::TerrainSdf;
use bevy::prelude::*;
use engine::SdfResource;
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
	let camera_pos = Vec3::new(0.0, 20.0, 30.0);
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
	));
}

pub fn camera_controller(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	terrain_sdf: Res<SdfResource<TerrainSdf>>,
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

	if controller.character_mode {
		// Character mode: gravity and terrain sticking
		character_mode_movement(
			&keyboard_input,
			&time,
			&terrain_sdf,
			&mut transform,
			&mut controller,
		);
	} else {
		// Free-fly mode: normal movement
		free_fly_movement(&keyboard_input, &time, &mut transform, &mut controller);
	}
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

fn character_mode_movement(
	keyboard_input: &Res<ButtonInput<KeyCode>>,
	time: &Res<Time>,
	terrain_sdf: &Res<SdfResource<TerrainSdf>>,
	transform: &mut Transform,
	controller: &mut CameraController,
) {
	const GRAVITY: f32 = -30.0; // Gravity acceleration (km/s²)
	const GROUND_STICK_DISTANCE: f32 = 0.0001; // Threshold for considering on ground (10cm)
	const CHARACTER_HEIGHT: f32 = 0.002; // Eye height above ground (2 meters)
	const CHARACTER_SPEED: f32 = 0.01; // Movement speed in character mode (10 m/s = 0.01 km/s)
	const JUMP_FORCE: f32 = 0.008; // Jump velocity (8 m/s = 0.008 km/s)
	const GROUND_FRICTION: f32 = 0.9; // Friction when on ground

	let dt = time.delta_secs();
	let pos = transform.translation;

	// Sample terrain height at current position (Box implements Deref, so we can call distance directly)
	let terrain_distance = terrain_sdf.sdf.sdf.distance(pos);
	let is_on_ground = terrain_distance <= GROUND_STICK_DISTANCE;

	// Apply gravity
	if !is_on_ground {
		controller.velocity.y += GRAVITY * dt;
	} else {
		// On ground: apply friction to horizontal velocity
		controller.velocity.x *= GROUND_FRICTION;
		controller.velocity.z *= GROUND_FRICTION;
		// Reset vertical velocity if on ground
		if controller.velocity.y < 0.0 {
			controller.velocity.y = 0.0;
		}
	}

	// Handle jump
	if keyboard_input.just_pressed(KeyCode::Space) && is_on_ground {
		controller.velocity.y = JUMP_FORCE;
	}

	// Handle horizontal movement
	let forward = transform.forward();
	let right = transform.right();
	let mut horizontal_movement = Vec3::ZERO;

	if keyboard_input.pressed(KeyCode::KeyW) {
		horizontal_movement += *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyS) {
		horizontal_movement -= *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyA) {
		horizontal_movement -= *right;
	}
	if keyboard_input.pressed(KeyCode::KeyD) {
		horizontal_movement += *right;
	}

	// Normalize horizontal movement and apply speed
	if horizontal_movement.length() > 0.0 {
		horizontal_movement.y = 0.0; // Remove vertical component
		horizontal_movement = horizontal_movement.normalize() * CHARACTER_SPEED;
		controller.velocity.x = horizontal_movement.x;
		controller.velocity.z = horizontal_movement.z;
	}

	// Apply velocity
	let new_pos = pos + controller.velocity * dt;

	// Find terrain height at new position
	let new_terrain_distance = terrain_sdf.sdf.sdf.distance(new_pos);

	// If we're going to be below ground or too close to it, stick to surface
	// Check if we're below surface (negative distance) or within character height
	if new_terrain_distance < CHARACTER_HEIGHT {
		// Use SDF distance directly: if distance is d at position (x, y, z),
		// the surface is at y - d. This is exact for vertical movement.
		let surface_height = new_pos.y - new_terrain_distance;
		let target_y = surface_height + CHARACTER_HEIGHT;

		// Smoothly move to target height (limit drop speed)
		let current_y = new_pos.y;
		let max_drop_per_frame = 0.005 * dt; // Don't drop faster than 5 m/s
		let target_y = target_y.max(current_y - max_drop_per_frame);

		// Update position: keep X and Z from movement, adjust Y to terrain
		transform.translation.x = new_pos.x;
		transform.translation.z = new_pos.z;
		transform.translation.y = target_y;

		// Reset vertical velocity if we hit the ground
		if new_terrain_distance <= GROUND_STICK_DISTANCE {
			controller.velocity.y = 0.0;
		}
	} else {
		transform.translation = new_pos;
	}
}
