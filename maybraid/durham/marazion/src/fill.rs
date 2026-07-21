//! Stamp-owned water fill for Marazion pocket bodies.
//!
//! Softmask sets the **horizontal** wet footprint. The volume itself is a
//! bleed-tolerant difference against terrain:
//!
//! \[
//! d = \max(y - W,\; (h - u) - y)
//! \]
//!
//! where \(u\) is [`WaterFill::terrain_undercut`]. Exact difference (`u = 0`)
//! has **no volume** wherever the rim sits above \(W\); undercut pushes water
//! into the bank so shoreline bleed is real, not only a wider softmask.

use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::{Region2D, RegionNoise};

/// Large positive distance used when a sample is outside the softmask boundary.
const OUTSIDE_FILL_DISTANCE: f32 = 1.0e6;

/// Stamp-owned water volume (Lake first).
///
/// Surface level, softmask, and terrain undercut are decided by the stamp;
/// [`Self::distance`] evaluates those products against a terrain height sample.
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Horizontal footprint (bowl + rim/apron bleed).
	pub region: Region2D,
	/// SDF-relative inset before the softmask fade begins (`0` = full wet to the SDF zero).
	pub inner_radius: f32,
	/// SDF-relative shore apron past the region boundary.
	pub outer_radius: f32,
	pub noise: Option<RegionNoise>,
	/// Water surface elevation decided by the stamp.
	pub water_level: f32,
	/// Push water into terrain by this many world units (`h_eff = h - undercut`).
	///
	/// Must be ≥ rim lift (plus a little) or the raised bank still blocks fill.
	pub terrain_undercut: f32,
}

impl WaterFill {
	/// Softmask weight in `[0, 1]` (`0` = fully inside / wet, `1` = outside / dry).
	pub fn softmask_at(&self, x: f32, z: f32) -> f32 {
		self.region.softmask_weight(
			Vec2::new(x, z),
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		)
	}

	/// Effective terrain height for the bleed-tolerant difference.
	pub fn effective_terrain_height(&self, terrain_height: f32) -> f32 {
		terrain_height - self.terrain_undercut.max(0.0)
	}

	/// Vertical wet interval `[h_eff, W]` when the column has volume under softmask.
	pub fn wet_y_span(&self, terrain_height: f32) -> Option<(f32, f32)> {
		let h_eff = self.effective_terrain_height(terrain_height);
		if self.water_level > h_eff {
			Some((h_eff, self.water_level))
		} else {
			None
		}
	}

	/// Softmask-clipped, bleed-tolerant water fill.
	///
	/// \[
	/// d = \max(y - W,\; (h - u) - y)
	/// \]
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		let w = self.softmask_at(p.x, p.z);
		if w >= 1.0 {
			return OUTSIDE_FILL_DISTANCE;
		}
		let h_eff = self.effective_terrain_height(terrain_height);
		let fill = (p.y - self.water_level).max(h_eff - p.y);
		fill + w * OUTSIDE_FILL_DISTANCE
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use jersey_terrain_stamps::CircleRegion;

	#[test]
	fn undercut_gives_volume_under_raised_rim() -> anyhow::Result<()> {
		let w = 40.0;
		let rim_h = w + 1.75;
		let fill = WaterFill {
			region: Region2D::Circle(CircleRegion {
				center: Vec2::ZERO,
				radius: 50.0,
			}),
			inner_radius: 0.0,
			outer_radius: 2.0,
			noise: None,
			water_level: w,
			terrain_undercut: 0.0,
		};
		// Exact difference: no volume when rim is above W.
		assert!(fill.wet_y_span(rim_h).is_none());
		let mid = Vec3::new(0.0, w - 0.5, 0.0);
		assert!(fill.distance(mid, rim_h) > 0.0);

		let bled = WaterFill {
			terrain_undercut: 3.0,
			..fill
		};
		let span = bled.wet_y_span(rim_h).expect("undercut volume");
		assert!(span.0 < w && span.1 == w);
		assert!(bled.distance(mid, rim_h) < 0.0);
		Ok(())
	}
}
