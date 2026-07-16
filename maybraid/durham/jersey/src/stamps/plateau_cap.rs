//! Jersey Plateau Caps (unchained) — [RFC-105 §3.8.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#382-jersey-plateau-caps-unchained).

use crate::config::{JitteredCenter};
use crate::modulation::{JerseyModulation, RegionAffineModulation, RegionGradingModulation};
use crate::region::{CircleRegion, RectRegion, Region2D, RegionNoise};
use crate::stamp::{StampSemantics, StampSet};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

/// Surface class for materials / props.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateauSurfaceClass {
	CapRock,
	SoilMantle,
}

/// Footprint style for the mesa / tableland.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateauFootprint {
	Blob,
	Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct PlateauCapParams {
	pub footprint: PlateauFootprint,
	pub surface: PlateauSurfaceClass,
	/// Cap radius / half-extent as a fraction of the shorter bound edge.
	pub size_frac: f32,
	/// Raise amount (world units) on the interior.
	pub lift: f32,
	/// Soft scale on base elevation under the cap.
	pub interior_scale: f32,
	/// Gentle tilt along +X of the bound (elevation delta across the cap).
	pub tilt: f32,
}

impl Default for PlateauCapParams {
	fn default() -> Self {
		Self {
			footprint: PlateauFootprint::Blob,
			surface: PlateauSurfaceClass::CapRock,
			size_frac: 0.28,
			lift: 18.0,
			interior_scale: 1.05,
			tilt: 4.0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct PlateauCap {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: PlateauCapParams,
	pub center: Vec2,
	pub stamp: StampSet,
}

impl PlateauCap {
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: PlateauCapParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let center = JitteredCenter::default().sample(bounds, seed, 11);
		let size = short * params.size_frac.clamp(0.1, 0.45);
		let rim_noise = RegionNoise::from_seed(seed.wrapping_add(3), 0.015, size * 0.08);

		let region = match params.footprint {
			PlateauFootprint::Blob => Region2D::Circle(CircleRegion { center, radius: size }),
			PlateauFootprint::Rect => {
				let aspect = 0.75 + 0.5 * hash.unit(5);
				Region2D::Rect(RectRegion {
					center,
					half_extents: Vec2::new(size * aspect, size / aspect.max(0.5)),
					round: size * 0.2,
				})
			}
		};

		let lift = params.lift;
		let affine = RegionAffineModulation::new(
			region.clone(),
			params.interior_scale,
			lift,
			size * 0.35,
			size * 0.85,
		)
		.with_noise(rim_noise.clone());

		let mut modulations = vec![JerseyModulation::Affine(affine)];
		// Optional tilt uses absolute grade targets from the height oracle; skip when
		// none is provided so we do not pull the cap toward elevation 0.
		if let Some(height_at) = height_at {
			let base_h = height_at(center.x, center.y);
			let tilt_axis =
				Vec2::new(1.0, 0.15 * (hash.unit(7) * 2.0 - 1.0)).normalize_or_zero();
			let start = center - tilt_axis * size * 0.7;
			let end = center + tilt_axis * size * 0.7;
			modulations.push(JerseyModulation::Grading(
				RegionGradingModulation::new(
					region,
					start,
					base_h + lift - params.tilt * 0.5,
					end,
					base_h + lift + params.tilt * 0.5,
					size * 0.3,
					size * 0.9,
				)
				.with_noise(rim_noise),
			));
		}

		let mut semantics = StampSemantics::default().with_tag("plateau").with_tag("escarpment");
		semantics = match params.surface {
			PlateauSurfaceClass::CapRock => semantics.with_tag("cap_rock"),
			PlateauSurfaceClass::SoilMantle => semantics.with_tag("soil_mantle"),
		};

		Self {
			bounds,
			seed,
			params,
			center,
			stamp: StampSet {
				modulations,
				spine: vec![center],
				semantics,
			},
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(
			bounds,
			seed,
			PlateauCapParams::default(),
			None,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plateau_lifts_interior() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let cap = PlateauCap::from_bounds_default(bounds, 9);
		let h_in = cap.stamp.apply_elevation(50.0, cap.center.x, cap.center.y);
		let h_out = cap.stamp.apply_elevation(50.0, bounds.min.x + 2.0, bounds.min.y + 2.0);
		assert!(h_in > h_out);
		assert!(cap.stamp.semantics.tags.contains(&"plateau"));
		Ok(())
	}
}
