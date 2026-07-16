//! Compose the Durham playground-style terrain SDF.

use crate::terrain::region::affine::RegionAffineModulation;
use crate::terrain::region::branching::BranchingPlan;
use crate::terrain::region::grading::RegionGradingModulation;
use crate::terrain::region::rounding::RegionRoundingModulation;
use crate::terrain::region::{CircleRegion, RectRegion, Region2D, RegionNoise};
use crate::terrain::sdf::{ComposedTerrain, TerrainSdf};
use bevy::prelude::*;
use sdf::{Ellipse3d, TubeSdf};

/// Configuration for terrain composition.
#[derive(Resource, Clone, Debug)]
pub struct TerrainConfig {
	pub seed: u32,
	pub height_scale: f32,
}

impl TerrainConfig {
	pub fn new(seed: u32) -> Self {
		Self { seed, height_scale: 5.0 }
	}
}

/// Create the composed terrain SDF (heightfield modulations + optional tube carve).
pub fn create_terrain(config: &TerrainConfig) -> ComposedTerrain {
	let mut sdf = TerrainSdf::new(config.seed, config.height_scale);

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
	.with_noise(RegionNoise::from_seed(config.seed, 0.2, 2.0));

	let intersecting_big_valley_sdf = RegionAffineModulation::new(
		Region2D::Circle(CircleRegion { center: Vec2::new(10.0, 70.0), radius: 80.0 }),
		0.5,
		-1.7,
		10.0,
		10.0,
	)
	.with_noise(RegionNoise::from_seed(config.seed, 0.2, 2.0));

	sdf.add_elevation_modulation(Box::new(intersecting_big_valley_sdf));

	let branch_plan = BranchingPlan::new(big_valley_sdf, config.seed, 5, 2);
	for modulation in branch_plan.generate_regions() {
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

	let tube_start = Vec3::new(-30.0, -1.0, -30.0);
	let tube_end = Vec3::new(-50.0, 4.0, -50.0);
	let tube_center = Vec3::new(20.0, 0.0, 20.0);
	let tube_axis = (tube_end - tube_start).normalize();

	let right = if tube_axis.x.abs() > tube_axis.z.abs() {
		Vec3::new(-tube_axis.y, tube_axis.x, 0.0).normalize()
	} else {
		Vec3::new(0.0, -tube_axis.z, tube_axis.y).normalize()
	};
	let up = tube_axis.cross(right).normalize();

	let tube_ellipse = Ellipse3d {
		center: tube_center,
		axes: [right, up],
		radii: Vec2::new(2.0, 2.0),
	};

	// Tube without surface noise (procedural-common migration path omits Perlin on util/sdf).
	let tube_sdf = TubeSdf::new(tube_start, tube_end, tube_ellipse);

	ComposedTerrain::from_terrain(sdf).with_tube(tube_sdf)
}
