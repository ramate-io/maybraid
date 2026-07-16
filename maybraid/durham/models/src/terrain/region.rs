pub mod affine;
pub mod branching;
pub mod grading;
pub mod rounding;

use bevy::prelude::*;
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

#[derive(Debug, Clone)]
pub struct ConvexPolyRegion {
	pub normals: Vec<Vec2>,
	pub offsets: Vec<f32>,
}

/// 2D region types with fast signed distance φ(x,z).
#[derive(Debug, Clone)]
pub enum Region2D {
	Rect(RectRegion),
	Circle(CircleRegion),
	ConvexPoly(ConvexPolyRegion),
}

/// Optional noise for perturbing region boundaries.
///
/// Holds a ready [`NoiseConfig`] because this type is internal to stamp
/// evaluation (not an authoring / CLI surface). Prefer constructing from
/// [`NoiseParams`] ([`Self::from_params`], [`Self::from_seed`]) when you need
/// the flexible param bundle; frequency and amplitude live there, and
/// [`Self::sample_boundary`] is a thin alias of [`NoiseConfig::sample_2d`].
#[derive(Clone)]
pub struct RegionNoise {
	pub noise: NoiseConfig,
}

impl std::fmt::Debug for RegionNoise {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RegionNoise")
			.field("noise_params", self.noise.params())
			.finish()
	}
}

impl RegionNoise {
	pub fn new(noise: NoiseConfig) -> Self {
		Self { noise }
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

	pub fn with_seed_offset(mut self, offset: i32) -> Self {
		let mut params = *self.noise.params();
		params.seed = params.seed.wrapping_add(offset);
		self.noise = NoiseConfig::new(params);
		self
	}

	pub fn sample_fbm(&self, x: f32, z: f32, amplitude: f32, frequency: f32) -> f32 {
		// Match legacy: accumulate octaves with amplitude ladder outside NoiseConfig amplitude.
		let mut value = 0.0;
		let mut amplitude_i = amplitude;
		let mut frequency_i = frequency;
		for _ in 0..4 {
			let octave = NoiseConfig::new(NoiseParams {
				seed: self.noise.params().seed,
				frequency: frequency_i,
				amplitude: 1.0,
				octaves: 1,
				noise_type: NoiseType::Perlin,
			});
			value += octave.sample_2d(Vec2::new(x, z)) * amplitude_i;
			amplitude_i *= 0.5;
			frequency_i *= 2.0;
		}
		value
	}

	pub fn sample_fbm_double_peak(&self, x: f32, z: f32, amplitude: f32, frequency: f32) -> f32 {
		let value = self.sample_fbm(x, z, amplitude, frequency);
		value.signum() * (amplitude - value.abs())
	}

	pub fn sample_boundary(&self, p: Vec2) -> f32 {
		self.noise.sample_2d(p)
	}
}

impl Region2D {
	pub fn convex_from_ccw_vertices(verts: &[Vec2]) -> Self {
		assert!(verts.len() >= 3);
		let mut normals = Vec::with_capacity(verts.len());
		let mut offsets = Vec::with_capacity(verts.len());
		for i in 0..verts.len() {
			let a = verts[i];
			let b = verts[(i + 1) % verts.len()];
			let e = b - a;
			let n = Vec2::new(e.y, -e.x).normalize();
			let b_i = -n.dot(a);
			normals.push(n);
			offsets.push(b_i);
		}
		Region2D::ConvexPoly(ConvexPolyRegion { normals, offsets })
	}

	#[inline(always)]
	pub fn sdf(&self, p: Vec2) -> f32 {
		self.sdf_with_noise(p, None)
	}

	pub fn is_inside(&self, p: Vec2) -> bool {
		self.sdf(p) < 0.0
	}

	#[inline(always)]
	pub fn sdf_with_noise(&self, p: Vec2, noise: Option<&RegionNoise>) -> f32 {
		let mut d = match self {
			Region2D::Rect(RectRegion { center, half_extents, round }) => {
				let q = (p - *center).abs() - *half_extents + Vec2::splat(*round);
				let outside = q.max(Vec2::ZERO).length() - *round;
				let inside = q.x.max(q.y).min(0.0);
				outside + inside
			}
			Region2D::Circle(CircleRegion { center, radius }) => (p - *center).length() - *radius,
			Region2D::ConvexPoly(ConvexPolyRegion { normals, offsets }) => {
				let mut m = -f32::INFINITY;
				for (n, b) in normals.iter().zip(offsets.iter()) {
					m = m.max(n.dot(p) + b);
				}
				m
			}
		};

		if let Some(noise_config) = noise {
			d += noise_config.sample_boundary(p);
		}

		d
	}

	pub fn relative_size(&self) -> f32 {
		match self {
			Region2D::Rect(RectRegion { half_extents, .. }) => half_extents.x,
			Region2D::Circle(CircleRegion { radius, .. }) => *radius,
			Region2D::ConvexPoly(ConvexPolyRegion { normals, .. }) => {
				normals.iter().map(|n| n.length()).fold(0.0_f32, f32::max)
			}
		}
	}

	pub fn num_vertices(&self) -> usize {
		match self {
			Region2D::ConvexPoly(ConvexPolyRegion { normals, .. }) => normals.len(),
			_ => 1,
		}
	}

	pub fn anchor_point(&self, index: usize) -> Vec2 {
		match self {
			Region2D::Rect(RectRegion { center, .. }) => *center,
			Region2D::Circle(CircleRegion { center, .. }) => *center,
			Region2D::ConvexPoly(ConvexPolyRegion { normals, offsets }) => {
				normals[index] + offsets[index] * normals[index]
			}
		}
	}

	pub fn branching_anchor_point(&self, noise: &RegionNoise) -> Vec2 {
		let relative_size = self.relative_size();
		let pow = (relative_size + 1317.0) * (relative_size + 1317.0);
		let anchor = self.anchor_point(0);
		let amplitude = (pow % relative_size) * 3.0;
		let x_offset =
			noise.sample_fbm_double_peak(anchor.x - 1.0, anchor.y + 1.0, amplitude, 0.05);
		let z_offset =
			noise.sample_fbm_double_peak(anchor.x + 1.0, anchor.y - 1.0, amplitude, 0.05);
		anchor + Vec2::new(x_offset, z_offset)
	}

	pub fn branching_scale(&self, noise: &RegionNoise) -> f32 {
		let anchor = self.anchor_point(0);
		let amplitude = 2.0;
		noise
			.sample_fbm_double_peak(anchor.x - 1.0, anchor.y + 1.0, amplitude, 0.05)
			.abs()
	}

	pub fn scale(&self, scale_body: f32, scale_detail: f32) -> Self {
		match self {
			Region2D::Rect(rect_region) => Region2D::Rect(RectRegion {
				half_extents: rect_region.half_extents * scale_body,
				round: rect_region.round * scale_detail,
				..rect_region.clone()
			}),
			Region2D::Circle(circle_region) => Region2D::Circle(CircleRegion {
				radius: circle_region.radius * scale_body,
				..circle_region.clone()
			}),
			Region2D::ConvexPoly(convex_poly_region) => Region2D::ConvexPoly(ConvexPolyRegion {
				normals: convex_poly_region.normals.iter().map(|n| n * scale_body).collect(),
				offsets: convex_poly_region.offsets.iter().map(|o| o * scale_body).collect(),
			}),
		}
	}

	pub fn reanchor(&self, anchor: Vec2) -> Self {
		match self {
			Region2D::Rect(rect_region) => {
				Region2D::Rect(RectRegion { center: anchor, ..rect_region.clone() })
			}
			Region2D::Circle(circle_region) => {
				Region2D::Circle(CircleRegion { center: anchor, ..circle_region.clone() })
			}
			Region2D::ConvexPoly(convex_poly_region) => Region2D::convex_from_ccw_vertices(
				&convex_poly_region.normals.iter().map(|n| n + anchor).collect::<Vec<Vec2>>(),
			),
		}
	}

	pub fn branch_region(&self, noise: &RegionNoise) -> Self {
		let anchor = self.branching_anchor_point(noise);
		let scale_body = self.branching_scale(noise);
		let scale_detail = self.branching_scale(noise);
		self.reanchor(anchor).scale(scale_body, scale_detail)
	}
}
