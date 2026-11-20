use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use terrain_shaders::TerrainShader;
use vegetation_grass_field::GrassPatch;

#[derive(Component)]
pub struct TurfPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<TerrainShader>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(80.0, 80.0));
	let material = materials.add(TerrainShader::default());
	commands.spawn((
		Name::new("turf-stand-in"),
		TurfPlane,
		Mesh3d(mesh),
		MeshMaterial3d(material.clone()),
		Transform::from_xyz(16.0, -0.04, 16.0),
		NotShadowCaster,
	));

	let slope = meshes.add(Plane3d::default().mesh().size(28.0, 28.0));
	commands.spawn((
		Name::new("slope-stand-in"),
		TurfPlane,
		Mesh3d(slope),
		MeshMaterial3d(material),
		Transform::from_xyz(32.0, 2.2, 6.0).with_rotation(Quat::from_rotation_z(-0.28)),
		NotShadowCaster,
	));

	commands.spawn((
		Name::new("gpu-grass-cover"),
		GrassPatch::dense_field(21, Vec3::new(-8.0, 0.0, -8.0), Vec2::splat(28.0), 0.18),
	));
}
