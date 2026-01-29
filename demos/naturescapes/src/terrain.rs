use crate::vegetation::ReadyForVegetation;
use bevy::prelude::*;
use chunk::cascade::Cascade;
use chunk::cascade::ConstantResolutionMap;
use chunk::cascade::DecreasingResolutionMap;
use noise::Perlin;
use render_item::lod::Lod;
use render_item::lod::LodPlugin;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::mesh::fetch_meshes;
use render_item::mesh::handle::MeshHandle;
use render_item::DispatchRenderItem;
use std::hash::Hash;
use terrain::region::affine::RegionAffineModulation;
use terrain::region::CircleRegion;
use terrain::region::RectRegion;
use terrain::{
	detail::meshes::rock::RockSpheroid,
	detail::meshes::tuft::GrassTuft,
	detail::terrain_detail::TerrainDetail,
	plugin::{Terrain, TerrainPlugin},
	region::branching::BranchingPlan,
	region::grading::RegionGradingModulation,
	region::rounding::RegionRoundingModulation,
	region::{Region2D, RegionNoise},
	render::TerrainRenderItem,
	Terrain as Terrainlike,
};

pub use terrain::TerrainSdf;

#[derive(Resource, Clone)]
pub struct TerrainMaterial<M: Material>(pub Handle<M>);

#[derive(Resource, Clone)]
pub struct TerrainResource<T: Terrainlike + Clone>(pub T);

#[derive(Resource, Clone)]
pub struct RockDetailMaterial<M: Material>(pub Handle<M>);

#[derive(Resource, Clone)]
pub struct SecondRockDetailMaterial<M: Material>(pub Handle<M>);

#[derive(Resource, Clone)]
pub struct TuftDetailMaterial<M: Material>(pub Handle<M>);

#[derive(Component, Clone)]
pub struct ReadyForRockDetail<T: Terrainlike + Clone>(pub T);

#[derive(Clone)]
pub struct TerrainPlaygroundPlugin<M: Material> {
	pub material: M,
	pub rock_detail_material: M,
	pub second_rock_detail_material: M,
	pub tuft_detail_material: M,
}

impl<M: Material> TerrainPlaygroundPlugin<M> {
	pub fn impl_setup_terrain_material(
		terrain_plaground_plugin: Self,
		mut commands: Commands,
		mut materials: ResMut<Assets<M>>,
	) {
		let material_handle = materials.add(terrain_plaground_plugin.material);
		commands.insert_resource(TerrainMaterial(material_handle));

		let rock_detail_material_handle =
			materials.add(terrain_plaground_plugin.rock_detail_material);
		commands.insert_resource(RockDetailMaterial(rock_detail_material_handle));

		let second_rock_detail_material_handle =
			materials.add(terrain_plaground_plugin.second_rock_detail_material);
		commands.insert_resource(SecondRockDetailMaterial(second_rock_detail_material_handle));

		let tuft_detail_material_handle =
			materials.add(terrain_plaground_plugin.tuft_detail_material);
		commands.insert_resource(TuftDetailMaterial(tuft_detail_material_handle));
	}

	pub fn build_setup_terrain_material(&self) -> impl FnMut(Commands, ResMut<Assets<M>>) {
		let me = self.clone();
		move |commands: Commands, materials: ResMut<Assets<M>>| {
			Self::impl_setup_terrain_material(me.clone(), commands, materials);
		}
	}

	pub fn setup_terrain(mut commands: Commands, terrain_material: Res<TerrainMaterial<M>>) {
		// Create base terrain SDF
		let mut sdf = TerrainSdf::new(42, 500.0);

		let big_valley_sdf = RegionAffineModulation::new(
			Region2D::Rect(RectRegion {
				center: Vec2::new(20.0, 20.0),
				half_extents: Vec2::new(90.0, 90.0),
				round: 2.0,
			}),
			0.5,
			0.0,
			10.0,
			10.0,
		)
		.with_noise(RegionNoise { noise: Perlin::new(42), frequency: 0.2, amplitude: 2.0 });

		let intersecting_big_valley_sdf = RegionAffineModulation::new(
			Region2D::Circle(CircleRegion { center: Vec2::new(10.0, 70.0), radius: 80.0 }),
			0.5,
			-1.7,
			10.0,
			10.0,
		)
		.with_noise(RegionNoise { noise: Perlin::new(42), frequency: 0.2, amplitude: 2.0 });

		sdf.add_elevation_modulation(Box::new(intersecting_big_valley_sdf));

		// branching regions
		let branch_plan = BranchingPlan::new(big_valley_sdf, Perlin::new(42), 5, 2);

		let modulations = branch_plan.generate_regions();

		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation));
		}

		let road_sdf = RegionRoundingModulation::new(
			Region2D::Rect(RectRegion {
				center: Vec2::new(0.0, 0.0),
				half_extents: Vec2::new(80.0, 1.0),
				round: 0.1,
			}),
			0.01,
			None,
			0.4,
			0.2,
		);

		sdf.add_elevation_modulation(Box::new(road_sdf));

		let start_point = Vec2::new(0.0, 20.0);
		let start_elevation = sdf.height_at_with_all_modulations(start_point.x, start_point.y);
		let end_point = Vec2::new(40.0, 20.0);
		let end_elevation = sdf.height_at_with_all_modulations(end_point.x, end_point.y);

		let graded_road = RegionGradingModulation::new(
			Region2D::Rect(RectRegion {
				center: Vec2::new(20.0, 20.0),
				half_extents: Vec2::new(20.0, 1.0),
				round: 0.01,
			}),
			start_point,
			start_elevation,
			end_point,
			end_elevation,
			None,
			0.4,
			0.1,
		);

		sdf.add_elevation_modulation(Box::new(graded_road));

		// insert the terrain resource so that dependendent systems can access it
		commands.spawn((ReadyForVegetation(sdf.clone()),));
		commands.spawn((ReadyForRockDetail(sdf.clone()),));

		// Set up the cascade
		let cascade = Cascade::<ConstantResolutionMap> {
			min_size: 20.0,
			number_of_rings: 0,
			resolution_map: ConstantResolutionMap { res_2: 5 },
			grid_radius: Some((12, 6, 12)),
			grid_multiple_2: 3,
		};

		let handle_map = HandleMap::<TerrainSdf>::new();
		let render_item = TerrainRenderItem::new(sdf, MeshMaterial3d(terrain_material.0.clone()))
			.with_handle_map(handle_map)
			.with_mesh_cache(DiskMeshCache::try_default().ok());

		commands.spawn((
			Terrain,
			Lod,
			cascade,
			Transform::from_translation(Vec3::ZERO),
			DispatchRenderItem::new(render_item),
			Children::default(),
		));
	}

	pub fn place_rock_detail(
		mut commands: Commands,
		rock_detail_material: Res<RockDetailMaterial<M>>,
		second_rock_detail_material: Res<SecondRockDetailMaterial<M>>,
		// this could also be an entity instead, but for now we'll make this oneshot
		ready_for_vegetation: Query<
			(Entity, &ReadyForVegetation<TerrainSdf>),
			Changed<ReadyForVegetation<TerrainSdf>>,
		>,
	) {
		for (_entity, ready_for_vegetation) in ready_for_vegetation.iter() {
			log::info!("Placing rocks");
			let rock_detail_cache = HandleMap::<RockSpheroid>::new();
			let rock_detail_mesh_cache = DiskMeshCache::try_default().ok();

			let cascade = Cascade {
				min_size: 15.0,
				number_of_rings: 5,
				resolution_map: DecreasingResolutionMap { from_res_2: 4, by: 1, min_res_2: 2 },
				grid_radius: None,
				grid_multiple_2: 0,
			};

			let terrain_detail = TerrainDetail::new(
				MeshMaterial3d(rock_detail_material.0.clone()),
				ready_for_vegetation.0.clone(),
			)
			.with_detail_handle_cache(rock_detail_cache.clone())
			.with_detail_mesh_cache(rock_detail_mesh_cache.clone());

			commands.spawn((
				Lod,
				cascade.clone(),
				DispatchRenderItem::new(terrain_detail),
				Transform::from_translation(Vec3::ZERO),
				Children::default(),
			));

			let second_terrain_detail = TerrainDetail::new(
				MeshMaterial3d(second_rock_detail_material.0.clone()),
				ready_for_vegetation.0.clone(),
			)
			.with_detail_handle_cache(rock_detail_cache)
			.with_detail_mesh_cache(rock_detail_mesh_cache)
			.with_step_size(Vec2::new(3.0, 3.0))
			.with_max_radii(Vec3::new(3.0, 3.0, 3.0));

			commands.spawn((
				Lod,
				cascade.clone(),
				DispatchRenderItem::new(second_terrain_detail),
				Transform::from_translation(Vec3::ZERO),
				Children::default(),
			));
		}
	}

	pub fn place_tuft_detail(
		mut commands: Commands,
		tuft_detail_material: Res<TuftDetailMaterial<M>>,
		ready_for_vegetation: Query<
			(Entity, &ReadyForVegetation<TerrainSdf>),
			Changed<ReadyForVegetation<TerrainSdf>>,
		>,
	) {
		for (_entity, ready_for_vegetation) in ready_for_vegetation.iter() {
			log::info!("Placing tufts");
			let tuft_detail_cache = HandleMap::<GrassTuft>::new();
			let tuft_detail_mesh_cache = DiskMeshCache::try_default().ok();

			let cascade = Cascade {
				min_size: 15.0,
				number_of_rings: 5,
				resolution_map: DecreasingResolutionMap { from_res_2: 4, by: 1, min_res_2: 2 },
				grid_radius: None,
				grid_multiple_2: 0,
			};

			let terrain_detail = TerrainDetail::new(
				MeshMaterial3d(tuft_detail_material.0.clone()),
				ready_for_vegetation.0.clone(),
			)
			.with_detail_handle_cache(tuft_detail_cache.clone())
			.with_detail_mesh_cache(tuft_detail_mesh_cache.clone())
			.with_sink_bias(2.0)
			.with_min_radii(Vec3::new(1.5, 2.5, 1.5))
			.with_max_radii(Vec3::new(3.0, 6.0, 3.0))
			.with_step_size(Vec2::new(1.0, 1.0));

			commands.spawn((
				Lod,
				cascade.clone(),
				DispatchRenderItem::new(terrain_detail),
				Transform::from_translation(Vec3::ZERO),
				Children::default(),
			));
		}
	}
}

impl<M: Material> Plugin for TerrainPlaygroundPlugin<M>
where
	M::Data: PartialEq + Eq + Hash + Clone,
{
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, self.build_setup_terrain_material());
		app.add_plugins(TerrainPlugin::<ConstantResolutionMap, M>::default());
		app.add_systems(Update, Self::setup_terrain.run_if(run_once));
		app.add_systems(Update, fetch_meshes::<MeshHandle<TerrainSdf>, M>);

		// rock detail
		app.add_plugins(LodPlugin::<
			DecreasingResolutionMap,
			TerrainDetail<RockSpheroid, M, TerrainSdf>,
		>::default());
		app.add_systems(Update, Self::place_rock_detail);
		app.add_systems(Update, fetch_meshes::<MeshHandle<RockSpheroid>, M>);

		// tuft detail
		app.add_plugins(LodPlugin::<
			DecreasingResolutionMap,
			TerrainDetail<GrassTuft, M, TerrainSdf>,
		>::default());
		app.add_systems(Update, Self::place_tuft_detail);
		app.add_systems(Update, fetch_meshes::<MeshHandle<GrassTuft>, M>);
	}
}
