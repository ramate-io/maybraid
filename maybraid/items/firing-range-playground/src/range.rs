//! Ground and static cube targets downrange (+X).

use avian3d::prelude::*;
use bevy::prelude::*;
use firearms::PenetrationCost;
use lod_avian::PhysicsInteractionLayer;

pub(crate) fn setup_range(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_ground(&mut commands, &mut meshes, &mut materials);
	spawn_cover(&mut commands, &mut meshes, &mut materials);
	spawn_targets(&mut commands, &mut meshes, &mut materials);
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
		PhysicsInteractionLayer::fixed_layers(),
		PenetrationCost(4.0),
	));
}

fn spawn_cover(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let crate_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.42, 0.34, 0.22),
		perceptual_roughness: 0.88,
		..default()
	});
	let wall_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.28, 0.3, 0.32),
		perceptual_roughness: 0.9,
		..default()
	});

	// Low crates: hip-height hide, eye-height sightline over the top.
	spawn_box(
		commands,
		meshes,
		crate_mat.clone(),
		"cover-crate-a",
		Vec3::new(4.5, 0.55, 1.2),
		Vec3::new(1.4, 1.1, 1.4),
	);
	spawn_box(
		commands,
		meshes,
		crate_mat,
		"cover-crate-b",
		Vec3::new(7.2, 0.55, -3.8),
		Vec3::new(1.3, 1.1, 1.3),
	);
	// Tall slabs: side-peek vantages.
	spawn_box(
		commands,
		meshes,
		wall_mat.clone(),
		"cover-wall-a",
		Vec3::new(3.2, 0.9, -1.4),
		Vec3::new(0.4, 1.8, 2.6),
	);
	spawn_box(
		commands,
		meshes,
		wall_mat,
		"cover-wall-b",
		Vec3::new(8.5, 0.9, 3.4),
		Vec3::new(0.4, 1.8, 2.4),
	);
}

fn spawn_box(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	material: Handle<StandardMaterial>,
	name: &'static str,
	translation: Vec3,
	size: Vec3,
) {
	commands.spawn((
		Name::new(name),
		Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
		MeshMaterial3d(material),
		Transform::from_translation(translation),
		RigidBody::Static,
		Collider::cuboid(size.x, size.y, size.z),
		PhysicsInteractionLayer::fixed_layers(),
		PenetrationCost(2.0),
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
			PhysicsInteractionLayer::fixed_layers(),
			PenetrationCost(1.0),
		));
	}
}
