use bevy::prelude::*;

use crate::checkerboard_material::CheckerboardMaterial;

/// Marks the ground plane so preview meshes can be cleaned up separately.
#[derive(Component)]
pub struct GroundPlane;

#[derive(Resource, Clone)]
pub struct PlaygroundSettings {
	pub seed: u32,
	pub checker_size_meters: f32,
}

impl Default for PlaygroundSettings {
	fn default() -> Self {
		Self { seed: 12345, checker_size_meters: 10.0 }
	}
}

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
	settings: Res<PlaygroundSettings>,
) {
	let size = 1000.0;
	let mesh = meshes.add(Plane3d::default().mesh().size(size, size));
	let material = materials.add(CheckerboardMaterial {
		checker_size_m: settings.checker_size_meters,
		color1: Color::srgb(0.9, 0.9, 0.9).into(),
		color2: Color::srgb(0.7, 0.7, 0.7).into(),
	});

	commands.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, 0.0, 0.0),
		GroundPlane,
	));
}
