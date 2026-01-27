use crate::shaders::refraction_water::RefractionWater;
use bevy::prelude::*;
use chunk::cascade::Cascade;
use chunk::cascade::ConstantResolutionMap;
use render_item::lod::Lod;
use render_item::mesh::fetch_meshes;
use render_item::mesh::handle::MeshHandle;
use render_item::DispatchRenderItem;
use terrain::ocean::{Ocean, OceanMesh, OceanPlugin};

#[derive(Resource, Clone)]
pub struct WaterMaterialHandle<M: Material>(pub Handle<M>);

pub fn setup_water(
	mut commands: Commands,
	mut refraction_materials: ResMut<Assets<RefractionWater>>,
) {
	let refraction_material_handle = refraction_materials.add(RefractionWater::default());
	commands.insert_resource(WaterMaterialHandle(refraction_material_handle));
}

pub fn water_playground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	refraction_material: Res<WaterMaterialHandle<RefractionWater>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

	// spawn an ocean cascade chunk at the origin
	let cascade = Cascade::<ConstantResolutionMap> {
		min_size: 1000.0,
		number_of_rings: 0,
		resolution_map: ConstantResolutionMap { res_2: 7 },
		grid_radius: (8, 8, 8),
		grid_multiple_2: 3,
	};

	commands.spawn((
		Lod,
		cascade,
		Transform::from_translation(Vec3::ZERO),
		DispatchRenderItem::new(Ocean::new(MeshMaterial3d::<RefractionWater>(
			refraction_material.0.clone(),
		))),
	));
}

pub struct WaterPlaygroundPlugin;

impl Plugin for WaterPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(bevy::pbr::MaterialPlugin::<RefractionWater>::default());
		app.add_systems(Startup, setup_water);
		app.add_systems(
			Update,
			water_playground
				.run_if(resource_exists::<WaterMaterialHandle<RefractionWater>>)
				.run_if(run_once),
		);
		app.add_plugins(OceanPlugin::<ConstantResolutionMap, RefractionWater>::default());
		app.add_systems(Update, fetch_meshes::<MeshHandle<OceanMesh>, RefractionWater>);
	}
}
