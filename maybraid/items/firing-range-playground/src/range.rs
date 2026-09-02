//! Ground pad under the Les Halles stack (courtyard is open to this slab).

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
}

fn spawn_ground(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let mesh = meshes.add(Cuboid::new(100.0, 0.2, 80.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.2, 0.17),
		perceptual_roughness: 0.92,
		..default()
	});
	commands.spawn((
		Name::new("ground"),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, -0.1, 0.0),
		RigidBody::Static,
		Collider::cuboid(100.0, 0.2, 80.0),
		PhysicsInteractionLayer::fixed_layers(),
		PenetrationCost(4.0),
	));
}
