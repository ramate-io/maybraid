use bevy::prelude::*;

/// Marks the infinite-looking ground plane so preview meshes can be cleaned up separately.
#[derive(Component)]
pub struct GroundPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let size = 1000.0;
	let mesh = meshes.add(Plane3d::default().mesh().size(size, size));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.85, 0.85, 0.88),
		perceptual_roughness: 0.85,
		..default()
	});

	commands.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, 0.0, 0.0),
		GroundPlane,
	));
}
