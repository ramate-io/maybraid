//! Noise-sampled I-frame knobs for an I-Apartment floor plan.

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{aabb_xz_extent, Confines, FitError};

/// Resolved I-frame layout knobs (stem + optional flanges).
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentParameterized {
	/// Stem width as a fraction of the short footprint axis.
	pub stem_width_frac: f32,
	/// Whether the top (−Z) flange bar is present.
	pub has_top_flange: bool,
	/// Whether the bottom (+Z) flange bar is present.
	pub has_bottom_flange: bool,
	/// Left/right flange length share of the leftover X budget (`0…1` each side).
	/// When a flange is present, lengths are `share * (extent_x - stem_w)`.
	pub top_left_share: f32,
	pub top_right_share: f32,
	pub bottom_left_share: f32,
	pub bottom_right_share: f32,
}

pub const MIN_STOREY_HEIGHT: f32 = 2.5;
pub const MIN_STEM_WIDTH: f32 = 4.0;
pub const MIN_FOOTPRINT: f32 = 12.0;

const SALT_STEM: f32 = 1.0;
const SALT_TOP: f32 = 2.0;
const SALT_BOT: f32 = 3.0;
const SALT_TL: f32 = 4.0;
const SALT_TR: f32 = 5.0;
const SALT_BL: f32 = 6.0;
const SALT_BR: f32 = 7.0;

impl IApartmentParameterized {
	/// Sample I-frame knobs at the confines center.
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let footprint = aabb_xz_extent(&confines.bounds);
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		if footprint.x < MIN_FOOTPRINT || footprint.y < MIN_FOOTPRINT {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let short = footprint.x.min(footprint.y);

		let stem_width_frac = cfg
			.sample_range_f32_4d(0.20, 0.40, c.x, c.y, c.z, SALT_STEM)
			.max(MIN_STEM_WIDTH / short.max(1.0))
			.min(0.55);

		// Prefer a full I; occasionally drop a flange → T / L / U-ish.
		let has_top_flange = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, SALT_TOP) > 0.18;
		let has_bottom_flange = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, SALT_BOT) > 0.18;
		if !has_top_flange && !has_bottom_flange {
			// Keep at least a stem rectangle (still 1 primary rect).
		}

		Ok(Self {
			stem_width_frac,
			has_top_flange,
			has_bottom_flange,
			top_left_share: cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, SALT_TL),
			top_right_share: cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, SALT_TR),
			bottom_left_share: cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, SALT_BL),
			bottom_right_share: cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, SALT_BR),
		})
	}

	/// Resolve concrete flange lengths for a footprint (meters).
	pub fn flange_lengths(
		&self,
		footprint: Vec2,
		stem_w: f32,
	) -> (Option<f32>, Option<f32>, Option<f32>, Option<f32>) {
		let leftover = (footprint.x - stem_w).max(0.0);
		let (tl, tr) = if self.has_top_flange {
			(
				Some(leftover * self.top_left_share),
				Some(leftover * self.top_right_share),
			)
		} else {
			(None, None)
		};
		let (bl, br) = if self.has_bottom_flange {
			(
				Some(leftover * self.bottom_left_share),
				Some(leftover * self.bottom_right_share),
			)
		} else {
			(None, None)
		};
		(tl, tr, bl, br)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn sample_accepts_large_footprint() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-20.0, 0.0, -16.0),
			Vec3::new(20.0, 3.5, 16.0),
		));
		let p = IApartmentParameterized::sample(&confines, NoiseParams::default()).unwrap();
		assert!(p.stem_width_frac > 0.0);
	}

	#[test]
	fn sample_rejects_tiny() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-2.0, 0.0, -2.0),
			Vec3::new(2.0, 3.0, 2.0),
		));
		assert!(matches!(
			IApartmentParameterized::sample(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}

