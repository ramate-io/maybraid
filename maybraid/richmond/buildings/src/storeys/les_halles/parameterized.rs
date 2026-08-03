//! Noise-sampled knobs for a Les Halles floor plan.

use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

/// Where vertical shafts are allocated in the ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LesHallesShaftPlacement {
	/// One shaft in each of the four corners of the gallery ring.
	Corners,
	/// One shaft in the middle of each of the four sides.
	MidSides,
}

/// Resolved Les Halles plan knobs (no geometry).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LesHallesParameterized {
	/// Outer commercial gallery band width (meters, full band depth on each side).
	pub gallery_width: f32,
	/// Inner balcony / walking band width (meters).
	pub balcony_width: f32,
	pub shaft_placement: LesHallesShaftPlacement,
	/// How densely to sprinkle extra gallery passages / apertures (`0…1`).
	pub opening_density: f32,
}

/// Minimum gallery band depth.
pub const MIN_GALLERY_WIDTH: f32 = 2.0;
/// Maximum gallery band depth sampled from noise.
pub const MAX_GALLERY_WIDTH: f32 = 4.5;
/// Minimum balcony band depth.
pub const MIN_BALCONY_WIDTH: f32 = 1.2;
/// Maximum balcony band depth sampled from noise.
pub const MAX_BALCONY_WIDTH: f32 = 2.5;
/// Minimum clear courtyard on each plan axis.
pub const MIN_COURTYARD: f32 = 2.0;
/// Minimum storey height.
pub const MIN_STOREY_HEIGHT: f32 = 2.5;
/// Nominal shaft side length (XZ).
pub const SHAFT_SIDE: f32 = 1.8;

/// Salt lanes for spatial sampling at the confines center.
const SALT_GALLERY: f32 = 1.0;
const SALT_BALCONY: f32 = 2.0;
const SALT_SHAFT: f32 = 3.0;
const SALT_OPENINGS: f32 = 4.0;

impl LesHallesParameterized {
	/// Sample knobs at the confines center. Rejects footprints that cannot host
	/// minimum gallery + balcony + courtyard on either axis.
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();

		let max_ring_x = ((extent_x - MIN_COURTYARD) * 0.5).max(min_ring);
		let max_ring_z = ((extent_z - MIN_COURTYARD) * 0.5).max(min_ring);
		let max_ring = max_ring_x.min(max_ring_z);

		let max_gallery = MAX_GALLERY_WIDTH.min(max_ring - MIN_BALCONY_WIDTH);
		let gallery_hi = max_gallery.max(MIN_GALLERY_WIDTH);
		let gallery_width =
			cfg.sample_range_f32_4d(MIN_GALLERY_WIDTH, gallery_hi, c.x, c.y, c.z, SALT_GALLERY);

		let max_balcony = MAX_BALCONY_WIDTH.min(max_ring - gallery_width);
		let balcony_hi = max_balcony.max(MIN_BALCONY_WIDTH);
		let balcony_width =
			cfg.sample_range_f32_4d(MIN_BALCONY_WIDTH, balcony_hi, c.x, c.y, c.z, SALT_BALCONY);

		// Re-check after sampling (noise ranges should already respect this).
		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_COURTYARD || extent_z < 2.0 * ring + MIN_COURTYARD {
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);

		Ok(Self {
			gallery_width,
			balcony_width,
			shaft_placement,
			opening_density,
		})
	}

	pub fn ring_width(&self) -> f32 {
		self.gallery_width + self.balcony_width
	}
}

pub(crate) fn footprint_extents(confines: &Confines) -> Result<(f32, f32, f32), FitError> {
	let min = confines.bounds.min;
	let max = confines.bounds.max;
	let extent_x = max.x - min.x;
	let extent_z = max.z - min.z;
	let height = max.y - min.y;
	if !extent_x.is_finite() || !extent_z.is_finite() || !height.is_finite() {
		return Err(FitError::InvalidConfines {
			reason: "non_finite_bounds",
		});
	}
	if extent_x <= 0.0 || extent_z <= 0.0 || height <= 0.0 {
		return Err(FitError::InvalidConfines {
			reason: "empty_bounds",
		});
	}
	Ok((extent_x, extent_z, height))
}
