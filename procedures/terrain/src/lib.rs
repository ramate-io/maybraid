pub mod ocean;
pub mod plugin;
pub mod region;
pub mod render;

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use noise::{NoiseFn, Perlin};
use render_item::{
	mesh::{IdentifiedMesh, MeshId},
	NormalizeChunk,
};
use sdf::{Sdf, Sign, SignBoundary, SignUniformIntervals};
use std::fmt::Debug;
use std::sync::Arc;

pub trait Terrain {
	fn composed_height_at(&self, x: f32, z: f32) -> f32;

	fn laplacian_at(&self, x: f32, z: f32, step: f32) -> f32 {
		let h0 = self.composed_height_at(x, z);
		let h1 = self.composed_height_at(x + step, z);
		let h2 = self.composed_height_at(x, z + step);
		let h3 = self.composed_height_at(x - step, z);
		let h4 = self.composed_height_at(x, z - step);
		(h1 + h2 + h3 + h4 - 4.0 * h0) / step / step
	}

	fn slope_mag(&self, x: f32, z: f32, step: f32) -> f32 {
		let hx1 = self.composed_height_at(x + step, z);
		let hx0 = self.composed_height_at(x - step, z);

		let hz1 = self.composed_height_at(x, z + step);
		let hz0 = self.composed_height_at(x, z - step);

		let dx = (hx1 - hx0) / (2.0 * step);
		let dz = (hz1 - hz0) / (2.0 * step);

		(dx * dx + dz * dz).sqrt()
	}

	fn slope_angle_deg(&self, x: f32, z: f32, step: f32) -> f32 {
		let s = self.slope_mag(x, z, step);
		s.atan().to_degrees()
	}
}

/// Trait for elevation modulations that modify terrain height in 2.5D
/// Returns the height offset at a given (x, z) position (Y is ignored)
pub trait ElevationModulation: Send + Sync + Debug {
	fn modify_elevation(
		&self,
		perlin_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		index: usize,
	) -> f32;
}

/// SDF representation of Perlin noise-based terrain
/// Converts the heightfield `y = height(x, z)` into an SDF: `f(p) = p.y - height(p.x, p.z)`
#[derive(Debug, Clone)]
pub struct TerrainSdf {
	/// The Perlin noise generator
	perlin: Perlin,
	/// The height scale
	height_scale: f32,
	/// The elevation modulations
	elevation_modulations: Vec<Arc<Box<dyn ElevationModulation>>>,
	/// Square describing bounds outside of which terrain is value 0
	bounds: Option<[Vec2; 4]>,
}

impl TerrainSdf {
	pub fn new(seed: u32, height_scale: f32) -> Self {
		Self {
			perlin: Perlin::new(seed),
			height_scale,
			elevation_modulations: Vec::new(),
			bounds: None,
		}
	}

	pub fn with_bounds(mut self, bounds: [Vec2; 4]) -> Self {
		self.bounds = Some(bounds);
		self
	}

	pub fn add_elevation_modulation(&mut self, modulation: Box<dyn ElevationModulation>) {
		self.elevation_modulations.push(Arc::new(modulation));
	}

	/// Calculate the terrain height at a given (x, z) position
	/// This is the same logic as the original heightfield generation
	fn height_at(&self, world_x: f32, world_z: f32) -> f32 {
		if let Some(bounds) = &self.bounds {
			if world_x < bounds[0].x
				|| world_x > bounds[1].x
				|| world_z < bounds[0].y
				|| world_z > bounds[1].y
			{
				return 0.0;
			}
		}

		// Generate height using multiple octaves of noise
		let mut height = 0.0;
		let mut amplitude = 1.0;
		let mut frequency = 0.0005;
		// let max_value = 0.0;

		for _ in 0..4 {
			let sample =
				self.perlin.get([world_x as f64 * frequency, world_z as f64 * frequency]) as f32;
			height += sample * amplitude;
			// max_value += amplitude;
			amplitude *= 0.5;
			frequency *= 2.0;
		}

		let exponent = 1.1; // >1 exaggerates contrast, <1 flattens
		let sign = height.signum();
		let height = sign * height.abs().powf(exponent);
		let height = height * self.height_scale;

		height
	}

	/*pub fn height_at_with_modulations_up_to(&self, world_x: f32, world_z: f32, index: usize) -> f32 {
		let mut terrain_height = self.height_at(world_x, world_z);
		for (i, modulation) in self.elevation_modulations[..index].iter().enumerate() {
			println!("modulation: {}, {:?}", i, modulation);
			terrain_height = modulation.modify_elevation(self, terrain_height, world_x, world_z, i);
		}
		terrain_height
	}*/

	pub fn height_at_with_all_modulations(&self, world_x: f32, world_z: f32) -> f32 {
		let mut terrain_height = self.height_at(world_x, world_z);
		for modulation in &self.elevation_modulations {
			terrain_height = modulation.modify_elevation(self, terrain_height, world_x, world_z, 0);
		}
		terrain_height
	}
}

impl Terrain for TerrainSdf {
	fn composed_height_at(&self, x: f32, z: f32) -> f32 {
		self.height_at_with_all_modulations(x, z)
	}
}

impl Sdf for TerrainSdf {
	fn distance(&self, p: Vec3) -> f32 {
		// Apply elevation modulations (2.5D height offsets)
		let terrain_height = self.height_at_with_all_modulations(p.x, p.z);

		// Define bedrock level (bottom of world)
		let bedrock_level = -self.height_scale * 4.0;

		// Distance to surface
		let d_surface = p.y - terrain_height;

		// Distance to bedrock (negative below bedrock)
		let d_bedrock = bedrock_level - p.y;

		// Take the maximum (intersection of half-spaces)
		// This keeps the interior solid between surface and bedrock.
		d_surface.max(d_bedrock)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		let mut intervals = SignUniformIntervals::default();

		// From below bedrock to the surface, we are outside the terrain,
		// so the sign is positive.
		intervals.insert_boundary(SignBoundary { min: f32::NEG_INFINITY, sign: Sign::Positive });

		// From bedrock to the surface, we are inside the terrain,
		// so the sign is negative.
		let bedrock_level = -self.height_scale * 4.0;
		intervals.insert_boundary(SignBoundary { min: bedrock_level, sign: Sign::Negative });

		// From the surface to infinity, we are outside the terrain,
		// so the sign is positive.
		let height = self.height_at_with_all_modulations(x, z);
		intervals.insert_boundary(SignBoundary { min: height, sign: Sign::Positive });

		intervals
	}
}

impl NormalizeChunk for TerrainSdf {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for TerrainSdf {
	fn id(&self) -> MeshId {
		let debug_string = format!("{:?}", self);
		MeshId::new(debug_string)
	}
}

#[derive(Debug, Clone)]
pub struct NullTerrain;

impl Terrain for NullTerrain {
	fn composed_height_at(&self, _x: f32, _z: f32) -> f32 {
		0.0
	}
}

impl Default for NullTerrain {
	fn default() -> Self {
		Self {}
	}
}
