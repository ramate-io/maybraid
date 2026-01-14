use bevy::prelude::*;
use buildings::{
	complex::{fillers::scratchpad::ScratchpadFiller, render::ComplexRenderer, Complex},
	meshes::walls::wall::WallMesh,
};
use chunk::cascade::CascadeChunk;
use engine::shaders::outline::EdgeMaterial;
use render_item::{mesh::cache::handle::map::HandleMap, DispatchRenderItem};

#[derive(Resource, Clone)]
pub struct BuildingMaterial<M: Material>(pub Handle<M>);

pub fn setup_buildings_material(
	mut commands: Commands,
	mut materials: ResMut<Assets<EdgeMaterial>>,
) {
	let material_handle = materials.add(EdgeMaterial {
		// brownish color
		base_color: Vec4::new(0.89, 0.886, 0.604, 1.0),
	});

	commands.insert_resource(BuildingMaterial(material_handle));
}

pub fn building_playground<F: Material, P: Material>(
	mut commands: Commands,
	_floor_material: Res<BuildingMaterial<F>>,
	partition_material: Res<BuildingMaterial<P>>,
) {
	log::info!("Spawning building playground");

	let partition_cache = HandleMap::<WallMesh>::new();
	let mut scratchpad_filler = ScratchpadFiller::new(MeshMaterial3d(partition_material.0.clone()))
		.with_wall_cache(partition_cache)
		.with_partition_threshold(0.4);
	let mut complex = Complex::new(Vec3::ZERO, Vec3::new(4.0, 2.0, 4.0), (16, 16, 16));
	complex.fill_canonical_members(&mut scratchpad_filler);
	let complex_renderer = ComplexRenderer::new(complex);

	commands.spawn((
		CascadeChunk::unit_center_chunk().with_res_2(3),
		DispatchRenderItem::new(complex_renderer),
		Transform::from_translation(Vec3::ZERO),
	));
}
