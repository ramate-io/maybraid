use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use terrain_shaders::TerrainShader;

#[derive(Component)]
pub struct TurfPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<TerrainShader>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(48.0, 48.0));
	let material = materials.add(TerrainShader::default());
	commands.spawn((
		Name::new("turf-stand-in"),
		TurfPlane,
		Mesh3d(mesh),
		MeshMaterial3d(material.clone()),
		Transform::from_xyz(12.0, -0.02, 12.0),
		NotShadowCaster,
	));

	let slope = meshes.add(Plane3d::default().mesh().size(16.0, 16.0));
	commands.spawn((
		Name::new("slope-stand-in"),
		TurfPlane,
		Mesh3d(slope),
		MeshMaterial3d(material),
		Transform::from_xyz(28.0, 1.6, 8.0).with_rotation(Quat::from_rotation_z(-0.28)),
		NotShadowCaster,
	));
}
