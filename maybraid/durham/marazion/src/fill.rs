//! Stamp-owned water fill for Marazion pocket bodies.
//!
//! Fills are “a water plane over terrain inside stamp bounds”: wet where
//! `terrain_height < water_level` under the region softmask. Softmask
//! `inner_radius` / `outer_radius` are Jersey **SDF-relative** apron widths
//! (see [`Region2D::softmask_weight`]), not absolute radii from the centroid.

use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::{Region2D, RegionNoise};

/// Large positive distance used when a sample is outside the softmask boundary.
const OUTSIDE_FILL_DISTANCE: f32 = 1.0e6;

/// Stamp-owned water volume (Lake first).
///
/// Surface level and softmask boundary are decided by the pocket-water stamp;
/// [`Self::distance`] only evaluates those products against a terrain height sample.
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Horizontal footprint (water disc only — not rim or apron).
	pub region: Region2D,
	/// SDF-relative inset before the softmask fade begins (`0` = full wet to the SDF zero).
	pub inner_radius: f32,
	/// SDF-relative shore apron past the region boundary.
	pub outer_radius: f32,
	pub noise: Option<RegionNoise>,
	/// Water surface elevation decided by the stamp.
	pub water_level: f32,
}

impl WaterFill {
	/// Softmask-clipped water fill: `Difference(BelowWaterPlane(W), Terrain)`.
	///
	/// \[
	/// d = \max(y - W,\; h - y)
	/// \]
	/// clipped by the stamp softmask so columns outside the water disc stay dry.
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		let w = self.region.softmask_weight(
			Vec2::new(p.x, p.z),
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		);
		if w >= 1.0 {
			return OUTSIDE_FILL_DISTANCE;
		}
		let fill = (p.y - self.water_level).max(terrain_height - p.y);
		fill + w * OUTSIDE_FILL_DISTANCE
	}
}
