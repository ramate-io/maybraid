//! Stamp-owned water fill for Marazion pocket bodies (lakes and streams).
//!
//! Softmask sets the **horizontal** wet footprint. A column is wet only when
//! \(W(x,z) > h - u\) (undercut lets shoreline / bank bleed under raised rims).
//! Inside a wet column the solid is a **half-space below the free surface**:
//!
//! \[
//! d = y - W(x,z)
//! \]
//!
//! (plus softmask fade). Authored leaves use [`WaterSurface::Hydro`]; [`WaterSurface::Flat`]
//! remains for unit tests of the fill half-space gate.

use crate::complex::HydrologyComplex;
use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::{Region2D, RegionNoise};

/// Large positive distance used when a sample is outside the softmask boundary
/// or the column is dry (\(W \le h_{\mathrm{eff}}\)).
const OUTSIDE_FILL_DISTANCE: f32 = 1.0e6;

/// Water surface elevation model decided by the stamp.
#[derive(Debug, Clone)]
pub enum WaterSurface {
	/// Constant lake (or pool) surface.
	Flat { level: f32 },
	/// Sample-time union surface from an indexed hydrology complex.
	Hydro { complex: HydrologyComplex },
}

impl WaterSurface {
	/// Surface elevation at a horizontal sample.
	pub fn level_at(&self, x: f32, z: f32) -> f32 {
		match self {
			Self::Flat { level } => *level,
			Self::Hydro { complex } => complex.surface_at(x, z).unwrap_or(0.0),
		}
	}
}

/// Stamp-owned water volume (Lake / Stream).
///
/// Surface, softmask, and terrain undercut are decided by the stamp;
/// [`Self::distance`] evaluates those products against a terrain height sample.
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Horizontal footprint (bowl / channel + bleed). Softmask for hydro fills
	/// is driven by [`WaterSurface::Hydro`]; region is a conservative proxy.
	pub region: Region2D,
	/// SDF-relative inset before the softmask fade begins (`0` = full wet to the SDF zero).
	pub inner_radius: f32,
	/// SDF-relative shore apron past the region boundary.
	pub outer_radius: f32,
	pub noise: Option<RegionNoise>,
	/// Water surface elevation model decided by the stamp.
	pub surface: WaterSurface,
	/// Push the wet-column gate into terrain (`h_eff = h - undercut`).
	///
	/// Does **not** set slab thickness — wet columns are half-spaces below \(W\).
	pub terrain_undercut: f32,
}

impl WaterFill {
	/// Representative horizontal samples for wet-volume gating.
	///
	/// Hydro fills probe authored node interiors (ellipse centers / reach midpoints)
	/// so a lake parked in a cell corner is not dropped when the region proxy
	/// (or a cell-covering circle) samples dry land at its centroid.
	pub fn wet_volume_probe_points(&self) -> Vec<Vec2> {
		match &self.surface {
			WaterSurface::Hydro { complex } => {
				let mut pts: Vec<Vec2> = complex
					.hydrology
					.iter()
					.map(|node| node.sample_point())
					.collect();
				if pts.is_empty() {
					pts.push(self.region.sample_point());
				}
				pts
			}
			WaterSurface::Flat { .. } => vec![self.region.sample_point()],
		}
	}

	/// Softmask weight in `[0, 1]` (`0` = fully inside / wet, `1` = outside / dry).
	pub fn softmask_at(&self, x: f32, z: f32) -> f32 {
		if let WaterSurface::Hydro { complex } = &self.surface {
			return complex.fill_softmask_at(x, z);
		}
		self.region.softmask_weight(
			Vec2::new(x, z),
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		)
	}

	/// Surface elevation \(W\) at `(x, z)`.
	pub fn surface_level_at(&self, x: f32, z: f32) -> f32 {
		self.surface.level_at(x, z)
	}

	/// Effective terrain height for the wet-column gate.
	pub fn effective_terrain_height(&self, terrain_height: f32) -> f32 {
		terrain_height - self.terrain_undercut.max(0.0)
	}

	/// Whether this column is wet under softmask + undercut (`W > h_eff`).
	pub fn column_is_wet(&self, x: f32, z: f32, terrain_height: f32) -> bool {
		self.surface_level_at(x, z) > self.effective_terrain_height(terrain_height)
	}

	/// Vertical wet interval when the column is wet: \((-\infty, W(x,z)]\).
	pub fn wet_y_span_at(&self, x: f32, z: f32, terrain_height: f32) -> Option<(f32, f32)> {
		if self.column_is_wet(x, z, terrain_height) {
			Some((f32::NEG_INFINITY, self.surface_level_at(x, z)))
		} else {
			None
		}
	}

	/// Softmask-clipped free-surface half-space (wet below \(W(x,z)\)).
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		let w_soft = self.softmask_at(p.x, p.z);
		if w_soft >= 1.0 {
			return OUTSIDE_FILL_DISTANCE;
		}
		if !self.column_is_wet(p.x, p.z, terrain_height) {
			return OUTSIDE_FILL_DISTANCE;
		}
		let fill = p.y - self.surface_level_at(p.x, p.z);
		fill + w_soft * OUTSIDE_FILL_DISTANCE
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
			surface: WaterSurface::Flat { level: w },
			terrain_undercut: 0.0,
		};
		assert!(fill.wet_y_span_at(0.0, 0.0, rim_h).is_none());
		let mid = Vec3::new(0.0, w - 0.5, 0.0);
		assert!(fill.distance(mid, rim_h) > 0.0);

		let bled = WaterFill {
			terrain_undercut: 3.0,
			..fill
		};
		let span = bled.wet_y_span_at(0.0, 0.0, rim_h).expect("undercut volume");
		assert!(span.0.is_infinite() && span.0.is_sign_negative());
		assert_eq!(span.1, w);
		assert!(bled.distance(mid, rim_h) < 0.0);
		Ok(())
	}
}
