//! Noise-sampled knobs for an I-Apartment floor plan.

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{aabb_xz_extent, Confines, FitError};

/// Resolved I-Apartment plan knobs.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentParameterized {
	/// Corridor clear width (meters).
	pub hall_width: f32,
	/// Spine offset along the short axis of each primary rect (`-0.5…0.5`,
	/// `0` = centered). Unique layouts without free Steiner trees.
	pub spine_offset: f32,
	/// Stem width of the I envelope as a fraction of the short footprint axis.
	pub stem_width_frac: f32,
	/// Minimum apartment room cell extents `(x, z)`.
	pub min_room_size: Vec2,
	/// Target total plan area for one apartment group.
	pub target_apartment_area: f32,
	/// Preferred janitorial closet side length (square target).
	pub janitorial_side: f32,
	/// Unit door clear width.
	pub unit_door_width: f32,
	/// Inter-rect portal / shaft-stub passage width.
	pub portal_width: f32,
}

pub const MIN_HALL_WIDTH: f32 = 1.4;
pub const MAX_HALL_WIDTH: f32 = 2.4;
pub const MIN_STOREY_HEIGHT: f32 = 2.5;
pub const MIN_STEM_WIDTH: f32 = 4.0;
pub const MIN_ROOM: f32 = 2.8;
pub const MAX_ROOM: f32 = 4.5;
pub const MIN_APT_AREA: f32 = 28.0;
pub const MAX_APT_AREA: f32 = 70.0;
pub const MIN_JANITORIAL_SIDE: f32 = 1.6;
pub const MAX_JANITORIAL_SIDE: f32 = 2.4;
pub const MIN_FOOTPRINT: f32 = 16.0;

const SALT_HALL: f32 = 1.0;
const SALT_SPINE: f32 = 2.0;
const SALT_STEM: f32 = 3.0;
const SALT_ROOM: f32 = 4.0;
const SALT_APT: f32 = 5.0;
const SALT_JAN: f32 = 6.0;
const SALT_DOOR: f32 = 7.0;

impl IApartmentParameterized {
	/// Sample knobs at the confines center.
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

		let hall_width = cfg.sample_range_f32_4d(
			MIN_HALL_WIDTH,
			MAX_HALL_WIDTH,
			c.x,
			c.y,
			c.z,
			SALT_HALL,
		);
		let spine_offset = cfg.sample_range_f32_4d(-0.35, 0.35, c.x, c.y, c.z, SALT_SPINE);
		let stem_width_frac = cfg
			.sample_range_f32_4d(0.22, 0.38, c.x, c.y, c.z, SALT_STEM)
			.min((short - 2.0 * MIN_HALL_WIDTH) / short.max(1.0))
			.max(MIN_STEM_WIDTH / short.max(1.0));
		let room = cfg.sample_range_f32_4d(MIN_ROOM, MAX_ROOM, c.x, c.y, c.z, SALT_ROOM);
		let target_apartment_area =
			cfg.sample_range_f32_4d(MIN_APT_AREA, MAX_APT_AREA, c.x, c.y, c.z, SALT_APT);
		let janitorial_side = cfg.sample_range_f32_4d(
			MIN_JANITORIAL_SIDE,
			MAX_JANITORIAL_SIDE,
			c.x,
			c.y,
			c.z,
			SALT_JAN,
		);
		let unit_door_width = cfg.sample_range_f32_4d(0.85, 1.1, c.x, c.y, c.z, SALT_DOOR);
		let portal_width = (unit_door_width + 0.15).clamp(0.95, 1.25);

		Ok(Self {
			hall_width,
			spine_offset,
			stem_width_frac,
			min_room_size: Vec2::splat(room),
			target_apartment_area,
			janitorial_side,
			unit_door_width,
			portal_width,
		})
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
		assert!(p.hall_width >= MIN_HALL_WIDTH);
		assert!(p.min_room_size.x >= MIN_ROOM - 1e-3);
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
