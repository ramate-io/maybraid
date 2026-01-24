use bevy::prelude::*;
use chunk::cascade::Cascade;
use chunk::cascade::ConstantResolutionMap;
use noise::Perlin;
use render_item::DispatchRenderItem;
use terrain_sdf::region::affine::RegionAffineModulation;
use terrain_sdf::region::CircleRegion;
use terrain_sdf::region::RectRegion;
use terrain_sdf::{
	plugin::TerrainPlugin,
	region::branching::BranchingPlan,
	region::grading::RegionGradingModulation,
	region::rounding::RegionRoundingModulation,
	region::{Region2D, RegionNoise},
	TerrainSdf,
};

pub struct TerrainPlaygroundPlugin;

impl Default for TerrainPlaygroundPlugin {
	fn default() -> Self {
		Self {}
	}
}

impl TerrainPlaygroundPlugin {
	pub fn setup_terrain(mut commands: Commands) {
		// Create base terrain SDF
		let mut sdf = TerrainSdf::new(42, 5.0);

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
			min_size: 10.0,
			number_of_rings: 10,
			resolution_map: ConstantResolutionMap { res_2: 10 },
			grid_radius: 10,
			grid_multiple_2: 10,
		};

		commands.spawn((
			Transform::from_translation(Vec3::ZERO),
			DispatchRenderItem::new(sdf),
			cascade,
		));
	}
}

impl Plugin for TerrainPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(TerrainPlugin::<ConstantResolutionMap>::default());
	}
}
