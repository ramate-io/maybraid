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
///
/// Holds a ready [`NoiseConfig`] because this type is internal to stamp
/// evaluation (not an authoring / CLI surface). Prefer constructing from
/// [`NoiseParams`] ([`Self::from_params`], [`Self::from_seed`]) when you need
/// the flexible param bundle; frequency and amplitude live there.
///
/// Set [`Self::expand_only`] so samples never shrink the geometric footprint
/// (`d += −|raw|`).
#[derive(Clone)]
pub struct RegionNoise {
	pub noise: NoiseConfig,
	/// When true, boundary samples only **expand** the region (never shrink).
	pub expand_only: bool,
}

impl std::fmt::Debug for RegionNoise {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RegionNoise")
			.field("noise_params", self.noise.params())
			.field("expand_only", &self.expand_only)
			.finish()
	}
}

impl RegionNoise {
	pub fn new(noise: NoiseConfig) -> Self {
		Self {
			noise,
			expand_only: false,
		}
	}

	pub fn from_params(params: NoiseParams) -> Self {
		Self::new(NoiseConfig::new(params))
	}

	pub fn from_seed(seed: u32, frequency: f32, amplitude: f32) -> Self {
		Self::from_params(NoiseParams {
			seed: seed as i32,
			frequency,
			amplitude,
			octaves: 1,
			noise_type: NoiseType::Perlin,
		})
	}

	/// Like [`Self::from_seed`], but boundary noise only expands the footprint.
	pub fn from_seed_expand_only(seed: u32, frequency: f32, amplitude: f32) -> Self {
		Self {
			expand_only: true,
			..Self::from_seed(seed, frequency, amplitude)
		}
	}

	pub fn expand_only(mut self) -> Self {
		self.expand_only = true;
		self
	}

	pub fn sample_boundary(&self, p: Vec2) -> f32 {
		let raw = self.noise.sample_2d(p);
		if self.expand_only {
			// `Region2D` does `d += sample`; negative sample expands.
			-raw.abs()
		} else {
			raw
		}
	}
}

impl Region2D {
	pub fn sdf(&self, p: Vec2) -> f32 {
		self.sdf_with_noise(p, None)
	}

	pub fn sdf_with_noise(&self, p: Vec2, noise: Option<&RegionNoise>) -> f32 {
		let mut d = match self {
			Region2D::Rect(RectRegion {
				center,
				half_extents,
				round,
			}) => {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expand_only_never_shrinks_circle() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion {
			center: Vec2::ZERO,
			radius: 10.0,
		});
		let noise = RegionNoise::from_seed_expand_only(7, 0.05, 2.0);
		for i in 0..32 {
			let ang = i as f32 * std::f32::consts::TAU / 32.0;
			let p = Vec2::new(ang.cos(), ang.sin()) * 10.0;
			let d0 = region.sdf(p);
			let d1 = region.sdf_with_noise(p, Some(&noise));
			assert!(
				d1 <= d0 + 1e-4,
				"expand-only must not increase SDF (shrink): d0={d0} d1={d1}"
			);
		}
		Ok(())
	}
}
