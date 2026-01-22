// use crate::geography::FeatureRegistry;
use crate::sdf::{Bounds, Difference, Ellipse3d, Sdf, SignUniformIntervals, TubeSdf};
use bevy::prelude::*;
use noise::Perlin;
use terrain_sdf::{
	region::affine::RegionAffineModulation,
	region::branching::BranchingPlan,
	region::grading::RegionGradingModulation,
	region::rounding::RegionRoundingModulation,
	region::{CircleRegion, RectRegion, Region2D, RegionNoise},
	PerlinTerrainSdf,
};

/// Resource containing the terrain SDF for runtime queries
#[derive(Resource)]
pub struct TerrainSdf {
	pub sdf: Box<dyn Sdf>,
}

impl Sdf for TerrainSdf {
	fn distance(&self, p: Vec3) -> f32 {
		self.sdf.distance(p)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		self.sdf.sign_uniform_on_y(x, z)
	}

	fn bounds(&self) -> Bounds {
		self.sdf.bounds()
	}
}

/// Create the terrain SDF with all modulations
pub fn create_terrain_sdf(config: &TerrainConfig) -> Box<dyn Sdf> {
	// Create base terrain SDF
	let mut sdf = PerlinTerrainSdf::new(config.seed, config.height_scale);

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
	.with_noise(RegionNoise { noise: Perlin::new(config.seed), frequency: 0.2, amplitude: 2.0 });

	let intersecting_big_valley_sdf = RegionAffineModulation::new(
		Region2D::Circle(CircleRegion { center: Vec2::new(10.0, 70.0), radius: 80.0 }),
		0.5,
		-1.7,
		10.0,
		10.0,
	)
	.with_noise(RegionNoise { noise: Perlin::new(config.seed), frequency: 0.2, amplitude: 2.0 });

	sdf.add_elevation_modulation(Box::new(intersecting_big_valley_sdf));

	// branching regions
	let branch_plan = BranchingPlan::new(big_valley_sdf, Perlin::new(config.seed), 5, 2);

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

	// Create a large vertical tube to bore a hole through the terrain
	// Position it near the origin, going from well below ground to well above
	let tube_start = Vec3::new(-30.0, -1.0, -30.0); // Start deep below
	let tube_end = Vec3::new(-50.0, 4.0, -50.0); // End high above

	// Create a circular cross-section (ellipse with equal radii)
	// Make it quite large - 15 unit radius
	let tube_center = Vec3::new(20.0, 0.0, 20.0);
	let tube_axis = (tube_end - tube_start).normalize();

	// Build orthonormal basis perpendicular to tube axis
	let right = if tube_axis.x.abs() > tube_axis.z.abs() {
		Vec3::new(-tube_axis.y, tube_axis.x, 0.0).normalize()
	} else {
		Vec3::new(0.0, -tube_axis.z, tube_axis.y).normalize()
	};
	let up = tube_axis.cross(right).normalize();

	let tube_ellipse = Ellipse3d {
		center: tube_center,
		axes: [right, up],
		radii: Vec2::new(2.0, 2.0), // Large circular cross-section
	};

	let tube_sdf = TubeSdf::new(tube_start, tube_end, tube_ellipse)
		.with_noise(Perlin::new(config.seed))
		.with_noise_factor(0.4);

	// Use Difference to bore the hole (subtract tube from terrain)
	Box::new(Difference::new(sdf, tube_sdf))
}

/// Configuration for terrain generation
#[derive(Resource, Clone)]
pub struct TerrainConfig {
	pub seed: u32,
	pub base_res_2: u8, // Full resolution vertices per chunk side
	pub height_scale: f32,
	pub use_volumetric: bool, // If true, use marching cubes; if false, use heightfield
}

impl TerrainConfig {
	pub fn new(seed: u32) -> Self {
		Self {
			seed,
			base_res_2: 7, // 128x128x128 voxels per chunk at full resolution
			height_scale: 5.0,
			use_volumetric: true, // Default to volumetric for true 3D terrain
		}
	}
}
