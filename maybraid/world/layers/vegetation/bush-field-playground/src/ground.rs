use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use terrain_shaders::TerrainShader;
use vegetation_grass_field::{GrassPatch, GrassSpecies};

#[derive(Component)]
pub struct TurfPlane;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<TerrainShader>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(36.0, 36.0));
	let material = materials.add(TerrainShader::default());
	commands.spawn((
		Name::new("turf-stand-in"),
		TurfPlane,
		Mesh3d(mesh),
		MeshMaterial3d(material.clone()),
		Transform::from_xyz(8.0, -0.02, 8.0),
		NotShadowCaster,
	));

	let slope = meshes.add(Plane3d::default().mesh().size(12.0, 12.0));
	commands.spawn((
		Name::new("slope-stand-in"),
		TurfPlane,
		Mesh3d(slope),
		MeshMaterial3d(material),
		Transform::from_xyz(18.0, 1.2, 4.0).with_rotation(Quat::from_rotation_z(-0.28)),
		NotShadowCaster,
	));
}

/// GPU grass from [#1641](https://github.com/ramate-io/maybraid/pull/1641) as meadow cover.
/// Height stays below small-bush crowns so wood and layered leaves stay readable.
pub fn setup_meadow(mut commands: Commands) {
	let cover = GrassSpecies::meadow().with_coverage_height(0.26);
	commands.spawn((
		Name::new("meadow-cover"),
		GrassPatch::dense_field(31, Vec3::new(-1.0, 0.0, -1.0), Vec2::splat(20.0), 0.13)
			.with_species(cover),
	));
	commands.spawn((
		Name::new("slope-meadow"),
		GrassPatch::sloped_field(
			37,
			Vec3::new(12.0, 0.3, -1.0),
			Vec2::splat(12.0),
			0.20,
			Vec3::new(-0.28, 1.0, 0.0),
		)
		.with_species(GrassSpecies::shade().with_coverage_height(0.30)),
	));
}
