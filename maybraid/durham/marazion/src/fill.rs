//! Stamp-owned water fill for Marazion pocket bodies (lakes first).
//!
//! Softmask sets the **horizontal** wet footprint. A column is wet only when
//! \(W > h - u\) (undercut lets shoreline bleed under raised rims). Inside a wet
//! column the solid is a **half-space below the free surface**:
//!
//! \[
//! d = y - W
//! \]
//!
//! (plus softmask fade). That matches terrain-style meshing on the shared
//! origin-cell cascade lattice: marching cubes resolves the free surface on the
//! same tall Y grid as terrain; subterranean volume is intentional and fine.
//! Islands / beds above \(W\) stay dry because the undercut gate fails.

use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::{Region2D, RegionNoise};

/// Large positive distance used when a sample is outside the softmask boundary
/// or the column is dry (\(W \le h_{\mathrm{eff}}\)).
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
	/// Push the wet-column gate into terrain (`h_eff = h - undercut`).
	///
	/// Must be ≥ rim lift (plus a little) or the raised bank still blocks fill.
	/// Does **not** set slab thickness — wet columns are half-spaces below \(W\).
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

	/// Effective terrain height for the wet-column gate.
	pub fn effective_terrain_height(&self, terrain_height: f32) -> f32 {
		terrain_height - self.terrain_undercut.max(0.0)
	}

	/// Whether this column is wet under softmask + undercut (`W > h_eff`).
	pub fn column_is_wet(&self, terrain_height: f32) -> bool {
		self.water_level > self.effective_terrain_height(terrain_height)
	}

	/// Vertical wet interval when the column is wet: \((-\infty, W]\).
	///
	/// Open downward so Durham water can share the terrain cell's full Y lattice.
	pub fn wet_y_span(&self, terrain_height: f32) -> Option<(f32, f32)> {
		if self.column_is_wet(terrain_height) {
			Some((f32::NEG_INFINITY, self.water_level))
		} else {
			None
		}
	}

	/// Softmask-clipped free-surface half-space (wet below \(W\)).
	///
	/// \[
	/// d = y - W
	/// \]
	///
	/// outside softmask or when \(W \le h_{\mathrm{eff}}\) → dry.
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		let w = self.softmask_at(p.x, p.z);
		if w >= 1.0 {
			return OUTSIDE_FILL_DISTANCE;
		}
		if !self.column_is_wet(terrain_height) {
			return OUTSIDE_FILL_DISTANCE;
		}
		let fill = p.y - self.water_level;
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
		// Exact gate: no volume when rim is above W.
		assert!(fill.wet_y_span(rim_h).is_none());
		let mid = Vec3::new(0.0, w - 0.5, 0.0);
		assert!(fill.distance(mid, rim_h) > 0.0);

		let bled = WaterFill {
			terrain_undercut: 3.0,
			..fill
		};
		let span = bled.wet_y_span(rim_h).expect("undercut volume");
		assert!(span.0.is_infinite() && span.0.is_sign_negative());
		assert_eq!(span.1, w);
		assert!(bled.distance(mid, rim_h) < 0.0);
		Ok(())
	}

	#[test]
	fn wet_column_is_half_space_below_surface() -> anyhow::Result<()> {
		let w = 50.0;
		let fill = WaterFill {
			region: Region2D::Circle(CircleRegion {
				center: Vec2::ZERO,
				radius: 40.0,
			}),
			inner_radius: 0.0,
			outer_radius: 1.0,
			noise: None,
			water_level: w,
			terrain_undercut: 2.0,
		};
		let bed = w - 5.0;
		assert!(fill.column_is_wet(bed));
		assert!(fill.distance(Vec3::new(0.0, w - 1000.0, 0.0), bed) < 0.0);
		assert!(fill.distance(Vec3::new(0.0, w + 1.0, 0.0), bed) > 0.0);
		Ok(())
	}
}
