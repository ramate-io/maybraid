//! 2D stamp footprints with optional boundary noise.

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

#[derive(Debug, Clone)]
pub struct RectRegion {
	pub center: Vec2,
	pub half_extents: Vec2,
	pub round: f32,
}

#[derive(Debug, Clone)]
pub struct CircleRegion {
	pub center: Vec2,
	pub radius: f32,
}

/// 2D region types with signed distance φ(x, z).
#[derive(Debug, Clone)]
pub enum Region2D {
	Rect(RectRegion),
	Circle(CircleRegion),
}

/// Optional noise for perturbing region boundaries (wobbly footprints).
#[derive(Clone)]
pub struct RegionNoise {
	pub noise: NoiseConfig,
	pub frequency: f32,
	pub amplitude: f32,
}

impl std::fmt::Debug for RegionNoise {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RegionNoise")
			.field("noise_params", self.noise.params())
			.field("frequency", &self.frequency)
			.field("amplitude", &self.amplitude)
			.finish()
	}
}

impl RegionNoise {
	pub fn from_seed(seed: u32, frequency: f32, amplitude: f32) -> Self {
		Self {
			noise: NoiseConfig::new(NoiseParams {
				seed: seed as i32,
				frequency: 1.0,
				amplitude: 1.0,
				octaves: 1,
				noise_type: NoiseType::Perlin,
			}),
			frequency,
			amplitude,
		}
	}

	pub fn sample_boundary(&self, p: Vec2) -> f32 {
		let config = NoiseConfig::new(NoiseParams {
			seed: self.noise.params().seed,
			frequency: self.frequency,
			amplitude: 1.0,
			octaves: 1,
			noise_type: NoiseType::Perlin,
		});
		config.sample_2d(p) * self.amplitude
	}
}

impl Region2D {
	pub fn sdf(&self, p: Vec2) -> f32 {
		self.sdf_with_noise(p, None)
	}

	pub fn sdf_with_noise(&self, p: Vec2, noise: Option<&RegionNoise>) -> f32 {
		let mut d = match self {
			Region2D::Rect(RectRegion { center, half_extents, round }) => {
				let q = (p - *center).abs() - *half_extents + Vec2::splat(*round);
				let outside = q.max(Vec2::ZERO).length() - *round;
				let inside = q.x.max(q.y).min(0.0);
				outside + inside
			}
			Region2D::Circle(CircleRegion { center, radius }) => (p - *center).length() - *radius,
		};
		if let Some(noise_config) = noise {
			d += noise_config.sample_boundary(p);
		}
		d
	}

	/// Softmask weight in `[0, 1]`: 0 deep inside, 1 outside the outer band.
	pub fn softmask_weight(
		&self,
		p: Vec2,
		inner_radius: f32,
		outer_radius: f32,
		noise: Option<&RegionNoise>,
	) -> f32 {
		let d = self.sdf_with_noise(p, noise);
		let outer = outer_radius.max(inner_radius + 0.001);
		if d < -inner_radius {
			0.0
		} else if d > outer {
			1.0
		} else {
			let t = (d + inner_radius) / (inner_radius + outer);
			smoothstep(t)
		}
	}
}

fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}
