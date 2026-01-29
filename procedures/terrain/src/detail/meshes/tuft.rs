use super::super::terrain_detail::MeshFromTerrainDetailNum;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use noise::{NoiseFn, Perlin};
use render_item::{
	mesh::{IdentifiedMesh, MeshId},
	NormalizeChunk,
};
use sdf::Sdf;

/// Configuration for a tuft of grass
#[derive(Debug, Clone)]
pub struct GrassTuftConfig {
	/// Seed for noise generation
	pub seed: u32,

	/// Number of blades in the tuft
	pub blade_count: u32,

	/// Height of each blade (unit space)
	pub blade_height: f32,

	/// Thickness of each blade
	pub blade_radius: f32,

	/// Spread of blades from the center
	pub tuft_radius: f32,

	/// Waviness strength
	pub noise_amplitude: f32,

	/// Noise frequency
	pub noise_frequency: f32,
}

impl Default for GrassTuftConfig {
	fn default() -> Self {
		Self {
			seed: 0,
			blade_count: 8,
			blade_height: 0.5,
			blade_radius: 0.07,
			tuft_radius: 0.3,
			noise_amplitude: 0.08,
			noise_frequency: 4.0,
		}
	}
}

/// Grass tuft SDF: a cluster of vertical noisy capsules
#[derive(Debug, Clone)]
pub struct GrassTuft {
	config: GrassTuftConfig,
	noise: Perlin,
}

impl GrassTuft {
	pub fn new(config: GrassTuftConfig) -> Self {
		let noise = Perlin::new(config.seed);
		Self { config, noise }
	}

	/// Distance to a vertical capsule (blade)
	fn capsule_distance(&self, p: Vec3, height: f32, radius: f32) -> f32 {
		// Clamp Y into blade segment
		let y = p.y.clamp(0.0, height);

		// Closest point on center line
		let closest = Vec3::new(0.0, y, 0.0);

		// Distance from capsule surface
		(p - closest).length() - radius
	}
}

impl Sdf for GrassTuft {
	fn distance(&self, p: Vec3) -> f32 {
		let mut min_dist = f32::MAX;

		// Each blade is a noisy capsule
		for i in 0..self.config.blade_count {
			let fi = i as f32;

			// Scatter blades radially using noise
			let angle = fi * 6.28318 / self.config.blade_count as f32;

			let offset = Vec3::new(
				angle.cos() * self.config.tuft_radius,
				0.0,
				angle.sin() * self.config.tuft_radius,
			);

			// Blade point in local space
			let mut blade_p = p - offset;

			// Add bending/waviness with noise
			let sway = self.noise.get([
				blade_p.x as f64 * self.config.noise_frequency as f64,
				blade_p.y as f64 * self.config.noise_frequency as f64,
				blade_p.z as f64 * self.config.noise_frequency as f64,
			]) as f32;

			blade_p.x += sway * self.config.noise_amplitude;
			blade_p.z += sway * self.config.noise_amplitude;

			// Capsule blade distance
			let d =
				self.capsule_distance(blade_p, self.config.blade_height, self.config.blade_radius);

			min_dist = min_dist.min(d);
		}

		min_dist
	}
}

impl NormalizeChunk for GrassTuft {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		CascadeChunk::unit_3d_center_chunk()
			.with_res_2(cascade_chunk.res_2)
			.with_mu(self.config.blade_radius + 0.1)
	}
}

impl IdentifiedMesh for GrassTuft {
	fn id(&self) -> MeshId {
		let debug_string = format!("{:?}", self);
		MeshId::new(debug_string)
	}
}

impl MeshFromTerrainDetailNum for GrassTuft {
	fn from_terrain_detail_num(_terrain_detail_num: f32) -> Self {
		Self::new(GrassTuftConfig::default())
	}
}
