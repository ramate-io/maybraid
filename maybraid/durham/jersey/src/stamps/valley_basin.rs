//! Jersey Valley Basins (unchained) — [RFC-105 §3.8.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#381-jersey-valley-basins-unchained).

use crate::modulation::{JerseyModulation, RegionAffineModulation, RegionGradingModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{StampSemantics, StampSet};
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

/// Valley cross-section family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValleyCrossSection {
	/// Sharp thalweg, steeper banks.
	V,
	/// Flatter floor, gentler walls.
	U,
	/// Bias the corridor to one bank.
	Asymmetric,
}

/// Floor readiness for a later hydrology stamp (height-only for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValleyFloorKind {
	/// Dry arroyo profile.
	Arroyo,
	/// Floor that a later water stamp may occupy.
	SpillwayReady,
}

/// Parameters for a single unchained valley basin inside a bound.
#[derive(Debug, Clone, Copy)]
pub struct ValleyBasinParams {
	pub cross_section: ValleyCrossSection,
	pub floor: ValleyFloorKind,
	/// Corridor half-width as a fraction of the shorter bound edge (`0.05..0.45`).
	pub width_frac: f32,
	/// Depression strength (world units of negative offset at the floor).
	pub depth: f32,
	/// Scale applied to base elevation inside the corridor (`< 1` softens relief).
	pub floor_scale: f32,
}

impl Default for ValleyBasinParams {
	fn default() -> Self {
		Self {
			cross_section: ValleyCrossSection::U,
			floor: ValleyFloorKind::SpillwayReady,
			width_frac: 0.18,
			depth: 12.0,
			floor_scale: 0.55,
		}
	}
}

/// Profile knobs derived from [`ValleyCrossSection`].
#[derive(Debug, Clone, Copy)]
struct CrossProfile {
	floor_scale: f32,
	depth: f32,
	bank_inner: f32,
	bank_outer: f32,
	width_scale: f32,
}

impl CrossProfile {
	fn from_params(params: &ValleyBasinParams) -> Self {
		match params.cross_section {
			ValleyCrossSection::V => Self {
				floor_scale: params.floor_scale * 0.85,
				depth: params.depth * 1.15,
				bank_inner: 0.15,
				bank_outer: 0.55,
				width_scale: 1.0,
			},
			ValleyCrossSection::U => Self {
				floor_scale: params.floor_scale,
				depth: params.depth,
				bank_inner: 0.35,
				bank_outer: 0.75,
				width_scale: 1.0,
			},
			ValleyCrossSection::Asymmetric => Self {
				floor_scale: params.floor_scale * 0.95,
				depth: params.depth,
				bank_inner: 0.25,
				bank_outer: 0.7,
				width_scale: 1.1,
			},
		}
	}
}

/// Constructed valley basin stamp (axis + height ops).
#[derive(Debug, Clone)]
pub struct ValleyBasin {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: ValleyBasinParams,
	pub start: Vec2,
	pub end: Vec2,
	pub path: Vec<Vec2>,
	pub stamp: StampSet,
}

impl ValleyBasin {
	/// Build a fractal-driven valley axis inside `bounds` and emit height modulations.
	///
	/// `height_at` samples the noise base for endpoint grade (typically durham
	/// `BaseTerrainNoise::height_at`). When `None`, endpoints use `0.0`.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: ValleyBasinParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let hash = SeededHash::new(seed);
		let extent = bounds.extent();
		let short = extent.min_element().max(1.0);
		let profile = CrossProfile::from_params(&params);

		// Low-frequency unit noise picks axis anchors so related seeds correlate
		// across neighboring bounds (fractal stamping placement, not pure cell dice).
		let start = bounds.project(Vec2::new(
			bounds.min.x + (0.12 + 0.28 * hash.unit(1)) * extent.x,
			bounds.min.y + (0.18 + 0.30 * hash.unit(2)) * extent.y,
		));
		let end = bounds.project(Vec2::new(
			bounds.min.x + (0.55 + 0.30 * hash.unit(3)) * extent.x,
			bounds.min.y + (0.50 + 0.32 * hash.unit(4)) * extent.y,
		));

		let step = (short * 0.08).clamp(8.0, 40.0);
		let config = HysteresisConfig {
			max_segments: 28,
			step_len: step,
			snap_radius: step * 0.75,
			connect_radius: step * 1.6,
			..HysteresisConfig::default()
		};
		let graph = HysteresisGraph::degree1(bounds, seed.wrapping_add(17), start, end, &config);
		let path = graph.primary_polyline();
		let a = *path.first().unwrap_or(&start);
		let b = *path.last().unwrap_or(&end);

		let half_width = short * params.width_frac.clamp(0.05, 0.45) * profile.width_scale;
		let axis = (b - a).normalize_or_zero();
		let perp = Vec2::new(-axis.y, axis.x);
		let side = if hash.unit(9) > 0.5 { 1.0 } else { -1.0 };
		let lateral = if params.cross_section == ValleyCrossSection::Asymmetric {
			perp * (half_width * 0.35 * side)
		} else {
			Vec2::ZERO
		};

		let h0 = height_at.map(|f| f(a.x, a.y)).unwrap_or(0.0);
		let h1 = height_at.map(|f| f(b.x, b.y)).unwrap_or(0.0);
		let (start_pt, start_h, end_pt, end_h) = if h0 >= h1 {
			(a, h0, b, h1)
		} else {
			(b, h1, a, h0)
		};

		let bank_noise = RegionNoise::from_seed(seed.wrapping_add(41), 0.02, half_width * 0.12);
		let inner_r = half_width * profile.bank_inner;
		let outer_r = half_width * profile.bank_outer;

		// Circle softmasks along the spine follow curved hysteresis paths better
		// than a single axis-aligned rect.
		let mut modulations = Vec::new();
		let sample_stride = ((path.len() / 6).max(1)).min(4);
		for (i, p) in path.iter().enumerate().step_by(sample_stride) {
			let center = *p + lateral;
			let region = Region2D::Circle(CircleRegion { center, radius: half_width });
			let depth_t = i as f32 / path.len().saturating_sub(1).max(1) as f32;
			let local_depth = profile.depth * (1.0 - 0.25 * depth_t);
			modulations.push(JerseyModulation::Affine(
				RegionAffineModulation::new(
					region,
					profile.floor_scale,
					-local_depth,
					inner_r,
					outer_r,
				)
				.with_noise(bank_noise.clone()),
			));
		}

		// One grading corridor on a widened circle at the midpoint for downhill bias.
		let grade_center = (start_pt + end_pt) * 0.5 + lateral;
		let grade_region = Region2D::Circle(CircleRegion {
			center: grade_center,
			radius: half_width * 1.35,
		});
		modulations.push(JerseyModulation::Grading(
			RegionGradingModulation::new(
				grade_region,
				start_pt,
				start_h - profile.depth * 0.35,
				end_pt,
				end_h - profile.depth * 0.15,
				inner_r * 0.8,
				outer_r * 1.1,
			)
			.with_noise(bank_noise),
		));

		let mut semantics = StampSemantics::default().with_tag("bank");
		semantics = match params.floor {
			ValleyFloorKind::Arroyo => semantics.with_tag("arroyo"),
			ValleyFloorKind::SpillwayReady => semantics.with_tag("spillway_ready"),
		};

		Self {
			bounds,
			seed,
			params,
			start: start_pt,
			end: end_pt,
			path: path.clone(),
			stamp: StampSet {
				modulations,
				spine: path,
				semantics,
			},
		}
	}

	/// Convenience: default params, no height oracle.
	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, ValleyBasinParams::default(), None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn valley_builds_spine_and_modulations() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let basin = ValleyBasin::from_bounds_default(bounds, 42);
		assert!(basin.path.len() >= 2);
		assert!(basin.stamp.modulations.len() >= 2);
		assert!(basin.stamp.semantics.tags.contains(&"bank"));
		assert!(basin.stamp.semantics.tags.contains(&"spillway_ready"));
		Ok(())
	}

	#[test]
	fn valley_depresses_center_relative_to_outside() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let basin = ValleyBasin::from_bounds(
			bounds,
			7,
			ValleyBasinParams {
				cross_section: ValleyCrossSection::V,
				floor: ValleyFloorKind::Arroyo,
				width_frac: 0.2,
				depth: 20.0,
				floor_scale: 0.5,
			},
			Some(&|_, _| 100.0),
		);
		let mid = (basin.start + basin.end) * 0.5;
		let outside = bounds.min + Vec2::new(5.0, 5.0);
		let h_mid = basin.stamp.apply_elevation(100.0, mid.x, mid.y);
		let h_out = basin.stamp.apply_elevation(100.0, outside.x, outside.y);
		assert!(h_mid < h_out);
		assert!(basin.stamp.semantics.tags.contains(&"arroyo"));
		Ok(())
	}
}
