use crate::shaders::refraction_water::RefractionWater;
use crate::shaders::water_material::WaterMaterial;
use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct WaterMaterialHandle<M: Material>(pub Handle<M>);

pub fn setup_water(
	mut commands: Commands,
	mut water_materials: ResMut<Assets<WaterMaterial>>,
	mut refraction_materials: ResMut<Assets<RefractionWater>>,
) {
	let water_material_handle = water_materials.add(WaterMaterial::default());
	commands.insert_resource(WaterMaterialHandle(water_material_handle));

	let refraction_material_handle = refraction_materials.add(RefractionWater::default());
	commands.insert_resource(WaterMaterialHandle(refraction_material_handle));
}

pub fn water_playground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	_water_material: Res<WaterMaterialHandle<WaterMaterial>>,
	refraction_material: Res<WaterMaterialHandle<RefractionWater>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	// spawn a big cube of water at the origin
	commands.spawn((
		Transform::from_translation(Vec3::ZERO),
		Mesh3d(meshes.add(Cuboid::new(10.0, 10.0, 10.0))),
		// MeshMaterial3d::<WaterMaterial>(water_material.0.clone()),
		MeshMaterial3d::<RefractionWater>(refraction_material.0.clone()),
	));

	// spawn a brown ball in standard material at the origin in the middle of the water
	commands.spawn((
		Transform::from_translation(Vec3::ZERO),
		Mesh3d(meshes.add(Sphere::new(1.0))),
		MeshMaterial3d::<StandardMaterial>(
			materials.add(StandardMaterial {
				base_color: Color::srgba(0.8, 0.4, 0.2, 1.0),
				..default()
			}),
		),
	));
}

pub struct WaterPlaygroundPlugin;

impl Plugin for WaterPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(bevy::pbr::MaterialPlugin::<RefractionWater>::default());
		app.add_plugins(bevy::pbr::MaterialPlugin::<WaterMaterial>::default());
		app.add_systems(Startup, setup_water);
		app.add_systems(
			Update,
			water_playground
				.run_if(resource_exists::<WaterMaterialHandle<WaterMaterial>>)
				.run_if(resource_exists::<WaterMaterialHandle<RefractionWater>>),
		);
	}
}
