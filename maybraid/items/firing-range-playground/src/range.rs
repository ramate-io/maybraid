//! Three bullpup stations: bolt, bullet, laser, aimed downrange (+X).

use avian3d::prelude::*;
use bevy::prelude::*;
use firearms::{firearm_bounds, spawn_firearm_components, FirearmConcept, FirearmKit, Weapon};
use std::f32::consts::FRAC_PI_2;

pub fn setup_range(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_ground(&mut commands, &mut meshes, &mut materials);
	spawn_targets(&mut commands, &mut meshes, &mut materials);

	let kit = FirearmConcept::Bullpup.kit();
	let aim = Quat::from_rotation_z(-FRAC_PI_2);
	let stations = [
		("bolt", Weapon::bolt(), Vec3::new(0.0, 1.25, -3.2)),
		("bullet", Weapon::bullet(), Vec3::new(0.0, 1.25, 0.0)),
		("laser", Weapon::laser(), Vec3::new(0.0, 1.25, 3.2)),
	];
	for (label, weapon, at) in stations {
		spawn_station(
			&mut commands,
			&kit,
			weapon,
			label,
			Transform { translation: at, rotation: aim, scale: Vec3::ONE },
		);
	}
}

fn spawn_station(
	commands: &mut Commands,
	kit: &FirearmKit,
	weapon: Weapon,
	label: &'static str,
	transform: Transform,
) {
	let bounds = firearm_bounds(kit);
	let entities = spawn_firearm_components(commands, kit, transform, bounds);
	for entity in entities {
		commands.entity(entity).insert((Name::new(label), weapon));
	}
}

fn spawn_ground(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let mesh = meshes.add(Cuboid::new(80.0, 0.2, 40.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.2, 0.17),
		perceptual_roughness: 0.92,
		..default()
	});
	commands.spawn((
		Name::new("ground"),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(20.0, -0.1, 0.0),
		RigidBody::Static,
		Collider::cuboid(80.0, 0.2, 40.0),
	));
}

fn spawn_targets(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let mesh = meshes.add(Cuboid::new(0.4, 1.6, 1.2));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.55, 0.22, 0.18),
		perceptual_roughness: 0.7,
		..default()
	});
	for (i, z) in [-3.2, 0.0, 3.2].into_iter().enumerate() {
		commands.spawn((
			Name::new(format!("target-{i}")),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material.clone()),
			Transform::from_xyz(18.0, 0.8, z),
			RigidBody::Static,
			Collider::cuboid(0.4, 1.6, 1.2),
		));
	}
}
