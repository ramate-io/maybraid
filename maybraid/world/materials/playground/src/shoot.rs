//! Playground-local repeating cylinders so energy shaders can be judged in motion.

use std::f32::consts::FRAC_PI_2;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use firearms::laser_hex_ref;
use material_ref::{MaterialRef, MaterialRefRoot};

pub const SHOT_INTERVAL: f32 = 0.35;
pub const SHOT_SPEED: f32 = 18.0;
pub const SHOT_MAX_AGE: f32 = 2.2;
pub const SHOT_RADIUS: f32 = 0.12;
pub const SHOT_LENGTH: f32 = 1.6;

#[derive(Resource, Clone, Debug)]
pub struct ShootConfig {
	pub enabled: bool,
	pub material: MaterialRef,
	pub interval: f32,
	pub accumulator: f32,
}

impl Default for ShootConfig {
	fn default() -> Self {
		Self {
			enabled: false,
			material: laser_hex_ref(),
			interval: SHOT_INTERVAL,
			accumulator: 0.0,
		}
	}
}

#[derive(Resource)]
pub struct ShotMesh(pub Handle<Mesh>);

#[derive(Component)]
pub struct Shot {
	pub age: f32,
}

pub fn tick_shoot(
	mut commands: Commands,
	time: Res<Time>,
	mut shoot: ResMut<ShootConfig>,
	mut meshes: ResMut<Assets<Mesh>>,
	mesh: Option<Res<ShotMesh>>,
) {
	if !shoot.enabled {
		shoot.accumulator = 0.0;
		return;
	}

	let handle = match mesh {
		Some(mesh) => mesh.0.clone(),
		None => {
			let handle = meshes.add(Mesh::from(Cylinder::new(SHOT_RADIUS, SHOT_LENGTH)));
			commands.insert_resource(ShotMesh(handle.clone()));
			handle
		}
	};

	shoot.accumulator += time.delta_secs();
	while shoot.accumulator >= shoot.interval {
		shoot.accumulator -= shoot.interval;
		spawn_shot(&mut commands, handle.clone(), shoot.material.clone());
	}
}

fn spawn_shot(commands: &mut Commands, mesh: Handle<Mesh>, material: MaterialRef) {
	commands.spawn((
		Name::new("material-shot"),
		Shot { age: 0.0 },
		Mesh3d(mesh),
		MaterialRefRoot(material),
		Transform {
			translation: Vec3::new(0.9, 1.0, 0.0),
			rotation: Quat::from_rotation_z(-FRAC_PI_2),
			scale: Vec3::ONE,
		},
		NotShadowCaster,
	));
}

pub fn fly_shots(
	mut commands: Commands,
	time: Res<Time>,
	mut shots: Query<(Entity, &mut Transform, &mut Shot)>,
) {
	let dt = time.delta_secs();
	for (entity, mut transform, mut shot) in &mut shots {
		shot.age += dt;
		transform.translation.x += SHOT_SPEED * dt;
		if shot.age >= SHOT_MAX_AGE {
			commands.entity(entity).try_despawn();
		}
	}
}
