use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use chunk::cascade::ConstantResolutionMap;
use render_item::lod::LodPlugin;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::fetch_meshes;
use render_item::mesh::handle::MeshHandle;
use render_item::DispatchRenderItem;
use std::hash::Hash;
use std::marker::PhantomData;
use terrain::Terrain as Terrainlike;
use vegetation_sdf::{
	grove::{Grove, GroveBuilder},
	tree::meshes::{canopy::ball::NoisyBall, trunk::segment::SimpleTrunkSegment},
};

#[derive(Resource, Clone)]
pub struct VegetationMaterial<M: Material>(pub Handle<M>);

#[derive(Component, Clone)]
pub struct ReadyForVegetation<T>(pub T);

#[derive(Clone)]
pub struct VegetationPlaygroundPlugin<T: Material, L: Material, E: Terrainlike + Clone> {
	pub trunk_material: T,
	pub leaf_material: L,
	__terrain_marker: PhantomData<E>,
}

impl<T: Material, L: Material, E: Terrainlike + Clone + Send + Sync + 'static>
	VegetationPlaygroundPlugin<T, L, E>
{
	pub fn new(trunk_material: T, leaf_material: L) -> Self {
		Self { trunk_material, leaf_material, __terrain_marker: PhantomData }
	}

	pub fn impl_setup_vegetation_materials(
		vegetation_playground_plugin: Self,
		mut commands: Commands,
		mut materials: ResMut<Assets<T>>,
		mut leaf_materials: ResMut<Assets<L>>,
	) {
		let trunk_material_handle = materials.add(vegetation_playground_plugin.trunk_material);
		let leaf_material_handle = leaf_materials.add(vegetation_playground_plugin.leaf_material);
		commands.insert_resource(VegetationMaterial(trunk_material_handle));
		commands.insert_resource(VegetationMaterial(leaf_material_handle));
	}

	pub fn build_setup_vegetation_materials(
		&self,
	) -> impl FnMut(Commands, ResMut<Assets<T>>, ResMut<Assets<L>>) {
		let me = self.clone();
		move |commands: Commands,
		      materials: ResMut<Assets<T>>,
		      leaf_materials: ResMut<Assets<L>>| {
			Self::impl_setup_vegetation_materials(me.clone(), commands, materials, leaf_materials);
		}
	}

	pub fn place_vegetation(
		mut commands: Commands,
		trunk_material: Res<VegetationMaterial<T>>,
		leaf_material: Res<VegetationMaterial<L>>,
		// this could also be an entity instead, but for now we'll make this oneshot
		ready_for_vegetation: Query<
			(Entity, &ReadyForVegetation<E>),
			Changed<ReadyForVegetation<E>>,
		>,
	) {
		for (_entity, ready_for_vegetation) in ready_for_vegetation.iter() {
			log::info!("Placing vegetation");
			let tree_cache = HandleMap::<SimpleTrunkSegment>::new();
			let leaf_cache = HandleMap::<NoisyBall>::new();

			let grove_builder = GroveBuilder::new(
				MeshMaterial3d(trunk_material.0.clone()),
				MeshMaterial3d(leaf_material.0.clone()),
				ready_for_vegetation.0.clone(),
			)
			.with_tree_cache(tree_cache)
			.with_leaf_cache(leaf_cache);

			commands.spawn((
				CascadeChunk::unit_center_chunk().with_res_2(3),
				DispatchRenderItem::new(grove_builder.build()),
				Transform::from_translation(Vec3::ZERO),
			));
		}
	}
}

impl<T: Material, L: Material, E: Terrainlike + Clone + Send + Sync + 'static> Plugin
	for VegetationPlaygroundPlugin<T, L, E>
where
	T::Data: PartialEq + Eq + Hash + Clone,
	L::Data: PartialEq + Eq + Hash + Clone,
{
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, self.build_setup_vegetation_materials());
		app.add_plugins(LodPlugin::<ConstantResolutionMap, Grove<T, L>>::default());
		app.add_systems(Update, Self::place_vegetation);
		app.add_systems(Update, fetch_meshes::<MeshHandle<SimpleTrunkSegment>, T>);
		app.add_systems(Update, fetch_meshes::<MeshHandle<NoisyBall>, L>);
	}
}
