//! Shared construction helpers for Jersey stamps.
//!
//! Each helper owns the knobs that only that operation needs. Stamp families
//! compose the helpers they use; there is no single umbrella config.

use crate::modulation::{JerseyModulation, RegionAffineModulation, RegionGradingModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

/// Deterministic start/end anchors as fractions of a bound's extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FractalAnchors {
	/// Minimum start position as a fraction of extent (XZ → `Vec2` xy).
	pub start_min_frac: Vec2,
	/// Additional random span for start (added to [`Self::start_min_frac`]).
	pub start_span_frac: Vec2,
	/// Minimum end position as a fraction of extent.
	pub end_min_frac: Vec2,
	/// Additional random span for end.
	pub end_span_frac: Vec2,
}

impl Default for FractalAnchors {
	fn default() -> Self {
		Self {
			start_min_frac: Vec2::new(0.12, 0.18),
			start_span_frac: Vec2::new(0.28, 0.30),
			end_min_frac: Vec2::new(0.55, 0.50),
			end_span_frac: Vec2::new(0.30, 0.32),
		}
	}
}

impl FractalAnchors {
	pub fn sample(&self, bounds: Bounds2, seed: u32, salt: u32) -> (Vec2, Vec2) {
		let hash = SeededHash::new(seed.wrapping_add(salt));
		let extent = bounds.extent();
		let start = bounds.project(Vec2::new(
			bounds.min.x
				+ (self.start_min_frac.x + self.start_span_frac.x * hash.unit(1)) * extent.x,
			bounds.min.y
				+ (self.start_min_frac.y + self.start_span_frac.y * hash.unit(2)) * extent.y,
		));
		let end = bounds.project(Vec2::new(
			bounds.min.x + (self.end_min_frac.x + self.end_span_frac.x * hash.unit(3)) * extent.x,
			bounds.min.y + (self.end_min_frac.y + self.end_span_frac.y * hash.unit(4)) * extent.y,
		));
		(start, end)
	}
}

/// Jittered placement of a single point inside a bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitteredCenter {
	/// Minimum center position as a fraction of extent.
	pub min_frac: Vec2,
	/// Random span for center (added to [`Self::min_frac`]).
	pub span_frac: Vec2,
}

impl Default for JitteredCenter {
	fn default() -> Self {
		Self {
			min_frac: Vec2::splat(0.25),
			span_frac: Vec2::splat(0.5),
		}
	}
}

impl JitteredCenter {
	pub fn sample(&self, bounds: Bounds2, seed: u32, salt: u32) -> Vec2 {
		let hash = SeededHash::new(seed.wrapping_add(salt));
		let extent = bounds.extent();
		bounds.project(Vec2::new(
			bounds.min.x + (self.min_frac.x + self.span_frac.x * hash.unit(1)) * extent.x,
			bounds.min.y + (self.min_frac.y + self.span_frac.y * hash.unit(2)) * extent.y,
		))
	}
}

/// Degree-1 hysteresis polyline from start toward end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisSpine {
	/// Path step length as a fraction of the shorter bound edge.
	pub step_frac: f32,
	/// Clamp for [`Self::step_frac`] × short edge (world units).
	pub step_min: f32,
	pub step_max: f32,
	/// Max segments in the walk.
	pub max_segments: usize,
	/// Snap-to-end radius as a multiple of step length.
	pub snap_step_mul: f32,
	/// Connect-to-end radius as a multiple of step length.
	pub connect_step_mul: f32,
}

impl Default for HysteresisSpine {
	fn default() -> Self {
		Self {
			step_frac: 0.08,
			step_min: 8.0,
			step_max: 40.0,
			max_segments: 28,
			snap_step_mul: 0.75,
			connect_step_mul: 1.6,
		}
	}
}

impl HysteresisSpine {
	pub fn walk_config(&self, bounds: Bounds2) -> HysteresisConfig {
		let short = bounds.extent().min_element().max(1.0);
		let step = (short * self.step_frac).clamp(self.step_min, self.step_max);
		HysteresisConfig {
			max_segments: self.max_segments,
			step_len: step,
			snap_radius: step * self.snap_step_mul,
			connect_radius: step * self.connect_step_mul,
			..HysteresisConfig::default()
		}
	}

	pub fn build(&self, bounds: Bounds2, seed: u32, start: Vec2, end: Vec2) -> Vec<Vec2> {
		HysteresisGraph::degree1(bounds, seed, start, end, &self.walk_config(bounds))
			.primary_polyline()
	}
}

/// Circle softmasks sampled along a polyline spine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftmaskAlongSpine {
	/// Target samples ≈ `len / stride_divisor` (then clamped).
	pub stride_divisor: usize,
	pub stride_min: usize,
	pub stride_max: usize,
	/// Depth/lift falls off by this fraction from head to tail of the spine.
	pub longitudinal_falloff: f32,
}

impl Default for SoftmaskAlongSpine {
	fn default() -> Self {
		Self {
			stride_divisor: 6,
			stride_min: 1,
			stride_max: 4,
			longitudinal_falloff: 0.2,
		}
	}
}

impl SoftmaskAlongSpine {
	/// Depression when `offset` is negative; lift when positive.
	pub fn build(
		&self,
		path: &[Vec2],
		half_width: f32,
		scale: f32,
		offset: f32,
		inner_frac: f32,
		outer_frac: f32,
		noise: &RegionNoise,
		lateral: Vec2,
	) -> Vec<JerseyModulation> {
		let mut out = Vec::new();
		if path.is_empty() {
			return out;
		}
		let stride = ((path.len() / self.stride_divisor.max(1)).max(self.stride_min))
			.min(self.stride_max);
		let inner_r = half_width * inner_frac;
		let outer_r = half_width * outer_frac;
		for (i, p) in path.iter().enumerate().step_by(stride) {
			let t = i as f32 / path.len().saturating_sub(1).max(1) as f32;
			let local = offset * (1.0 - self.longitudinal_falloff * t);
			let region = Region2D::Circle(CircleRegion {
				center: *p + lateral,
				radius: half_width,
			});
			out.push(JerseyModulation::Affine(
				RegionAffineModulation::new(region, scale, local, inner_r, outer_r)
					.with_noise(noise.clone()),
			));
		}
		out
	}
}

/// Single grading region centered between two endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidpointGrading {
	/// Grading region radius as a multiple of corridor half-width.
	pub radius_half_width_mul: f32,
	/// Softmask inner radius as a fraction of corridor half-width.
	pub inner_half_width_frac: f32,
	/// Softmask outer radius as a fraction of corridor half-width.
	pub outer_half_width_frac: f32,
}

impl Default for MidpointGrading {
	fn default() -> Self {
		Self {
			radius_half_width_mul: 1.35,
			inner_half_width_frac: 0.25,
			outer_half_width_frac: 0.85,
		}
	}
}

impl MidpointGrading {
	pub fn build(
		&self,
		start: Vec2,
		start_h: f32,
		end: Vec2,
		end_h: f32,
		half_width: f32,
		noise: RegionNoise,
	) -> JerseyModulation {
		let center = (start + end) * 0.5;
		let region = Region2D::Circle(CircleRegion {
			center,
			radius: half_width * self.radius_half_width_mul,
		});
		JerseyModulation::Grading(
			RegionGradingModulation::new(
				region,
				start,
				start_h,
				end,
				end_h,
				half_width * self.inner_half_width_frac,
				half_width * self.outer_half_width_frac,
			)
			.with_noise(noise),
		)
	}
}

/// Order endpoints so grade runs downhill when heights are available.
///
/// Stateless utility (no tunables).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DownhillPair;

impl DownhillPair {
	pub fn order(
		a: Vec2,
		b: Vec2,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> (Vec2, f32, Vec2, f32) {
		let h0 = height_at.map(|f| f(a.x, a.y)).unwrap_or(0.0);
		let h1 = height_at.map(|f| f(b.x, b.y)).unwrap_or(0.0);
		if h0 >= h1 {
			(a, h0, b, h1)
		} else {
			(b, h1, a, h0)
		}
	}
}
