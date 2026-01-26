use bevy::prelude::*;
use chunk::cascade::Cascade;
use chunk::cascade::ConstantResolutionMap;
use noise::Perlin;
use render_item::lod::Lod;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::fetch_meshes;
use render_item::mesh::handle::MeshHandle;
use render_item::DispatchRenderItem;
use std::hash::Hash;
use terrain_sdf::region::affine::RegionAffineModulation;
use terrain_sdf::region::CircleRegion;
use terrain_sdf::region::RectRegion;
use terrain_sdf::{
	plugin::{Terrain, TerrainPlugin},
	region::branching::BranchingPlan,
	region::grading::RegionGradingModulation,
	region::rounding::RegionRoundingModulation,
	region::{Region2D, RegionNoise},
	render::TerrainRenderItem,
	TerrainSdf,
};

#[derive(Resource, Clone)]
pub struct TerrainMaterial<M: Material>(pub Handle<M>);

#[derive(Clone)]
pub struct TerrainPlaygroundPlugin<M: Material> {
	pub material: M,
}

impl<M: Material> TerrainPlaygroundPlugin<M> {
	pub fn impl_setup_terrain_material(
		terrain_plaground_plugin: Self,
		mut commands: Commands,
		mut materials: ResMut<Assets<M>>,
	) {
		let material_handle = materials.add(terrain_plaground_plugin.material);
		commands.insert_resource(TerrainMaterial(material_handle));
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

		// Set up the cascade
		let cascade = Cascade::<ConstantResolutionMap> {
			min_size: 20.0,
			number_of_rings: 0,
			resolution_map: ConstantResolutionMap { res_2: 5 },
			grid_radius: 12,
			grid_multiple_2: 3,
		};

		let handle_map = HandleMap::<TerrainSdf>::new();
		let render_item = TerrainRenderItem::new(sdf, MeshMaterial3d(terrain_material.0.clone()))
			.with_handle_map(handle_map);

		commands.spawn((
			Terrain,
			Lod,
			cascade,
			Transform::from_translation(Vec3::ZERO),
			DispatchRenderItem::new(render_item),
			Children::default(),
		));
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
	}
}
