use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use engine::shaders::{leaf_material::LeafMaterial, outline::EdgeMaterial};
use render_item::{mesh::cache::handle::map::HandleMap, DispatchRenderItem};
use vegetation_sdf::{
	grove::GroveBuilder,
	tree::{
		meshes::{canopy::ball::NoisyBall, trunk::segment::SimpleTrunkSegment},
		TreeRenderItem,
	},
};

#[derive(Resource, Clone)]
pub struct TreeMaterial<M: Material>(pub Handle<M>);

pub fn setup_tree_edge_material(
	mut commands: Commands,
	mut materials: ResMut<Assets<EdgeMaterial>>,
	mut leaf_materials: ResMut<Assets<LeafMaterial>>,
) {
	let material_handle = materials.add(EdgeMaterial {
		// brownish color
		base_color: Vec4::new(0.89, 0.886, 0.604, 1.0),
	});

	// green color
	let leaf_material_handle =
		leaf_materials.add(LeafMaterial { base_color: Vec4::new(0.2, 0.8, 0.3, 1.0) });

	commands.insert_resource(TreeMaterial(material_handle));
	commands.insert_resource(TreeMaterial(leaf_material_handle));
}

pub fn tree_playground<T: Material, L: Material>(
	mut commands: Commands,
	trunk_material: Res<TreeMaterial<T>>,
	leaf_material: Res<TreeMaterial<L>>,
) {
	log::info!("Spawning tree playground");

	let tree_cache = HandleMap::<SimpleTrunkSegment>::new();
	let leaf_cache = HandleMap::<NoisyBall>::new();

	let grove_builder = GroveBuilder::new(
		MeshMaterial3d(trunk_material.0.clone()),
		MeshMaterial3d(leaf_material.0.clone()),
	)
	.with_tree_cache(tree_cache)
	.with_leaf_cache(leaf_cache);
	let grove = grove_builder.build();

	commands.spawn((
		CascadeChunk::unit_center_chunk().with_res_2(3),
		DispatchRenderItem::new(grove),
		Transform::from_translation(Vec3::ZERO),
	));
}

pub fn square_tree_playground<T: Material, L: Material>(
	mut commands: Commands,
	trunk_material: Res<TreeMaterial<T>>,
	leaf_material: Res<TreeMaterial<L>>,
) {
	log::info!("Spawning tree playground");

	let tree_cache = HandleMap::<SimpleTrunkSegment>::new();
	let leaf_cache = HandleMap::<NoisyBall>::new();

	// grid out some trees
	const N: i32 = 4;
	for x in -N..=N {
		for z in -N..=N {
			tree(
				&mut commands,
				Vec3::new(x as f32 * 4.0, 0.0, z as f32 * 4.0),
				&trunk_material,
				&leaf_material,
				tree_cache.clone(),
				leaf_cache.clone(),
			);
		}
	}
}

pub fn tree<T: Material, L: Material>(
	commands: &mut Commands,
	origin: Vec3,
	trunk_material: &Res<TreeMaterial<T>>,
	leaf_material: &Res<TreeMaterial<L>>,
	tree_cache: HandleMap<SimpleTrunkSegment>,
	leaf_cache: HandleMap<NoisyBall>,
) {
	commands.spawn((
		CascadeChunk::unit_center_chunk().with_res_2(3),
		DispatchRenderItem::new(
			TreeRenderItem::new(
				MeshMaterial3d(trunk_material.0.clone()),
				MeshMaterial3d(leaf_material.0.clone()),
			)
			.with_tree_cache(tree_cache.clone())
			.with_leaf_cache(leaf_cache.clone()),
		),
		Transform::from_translation(origin),
	));
}
