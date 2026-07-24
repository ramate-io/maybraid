//! Stamp-owned water fill for Marazion pocket bodies (lakes and streams).
//!
//! Hydro fills delegate to [`HydrologyComplex::water_distance`]: outside the carve,
//! approximate distance to \(\phi = 0\); inside, the vertical slab between terrain
//! \(h\) and blended free surface \(W\). Flat fills (unit tests) use the same slab
//! idea against a circle/region SDF.

use crate::complex::HydrologyComplex;
use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::Region2D;

/// Water surface elevation model decided by the stamp.
#[derive(Debug, Clone)]
pub enum WaterSurface {
	/// Constant lake (or pool) surface — tests / simple stamps.
	Flat { level: f32 },
	/// Indexed hydrology complex (owns \(W\) blend + water SDF).
	Hydro { complex: HydrologyComplex },
}

impl WaterSurface {
	/// Surface elevation at a horizontal sample (`0` when dry / undefined).
	pub fn level_at(&self, x: f32, z: f32) -> f32 {
		match self {
			Self::Flat { level } => *level,
			Self::Hydro { complex } => complex.surface_at(x, z).unwrap_or(0.0),
		}
	}
}

/// Stamp-owned water volume (Lake / Stream).
///
/// Hydro: [`Self::distance`] delegates to [`HydrologyComplex::water_distance`].
/// Flat: region SDF ∩ slab \([h, W]\).
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Horizontal footprint proxy (probes / Flat SDF). Hydro soft-support is \(\phi\).
	pub region: Region2D,
	/// Water surface elevation model decided by the stamp.
	pub surface: WaterSurface,
}

impl WaterFill {
	/// Representative horizontal samples for wet-volume gating.
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

	/// Surface elevation \(W\) at `(x, z)`.
	pub fn surface_level_at(&self, x: f32, z: f32) -> f32 {
		self.surface.level_at(x, z)
	}

	/// True when the XZ sample is inside the hydro carve (or Flat region).
	pub fn inside_horizontal(&self, x: f32, z: f32) -> bool {
		match &self.surface {
			WaterSurface::Hydro { complex } => complex.inside_carve(x, z),
			WaterSurface::Flat { .. } => self.region.sdf(Vec2::new(x, z)) <= 0.0,
		}
	}

	/// Vertical wet interval when the column is wet: \((h, W]\) with \(W > h\).
	pub fn wet_y_span_at(&self, x: f32, z: f32, terrain_height: f32) -> Option<(f32, f32)> {
		if !self.inside_horizontal(x, z) {
			return None;
		}
		let w = self.surface_level_at(x, z);
		if w > terrain_height {
			Some((terrain_height, w))
		} else {
			None
		}
	}

	/// Water SDF at `p` given composed terrain height `terrain_height`.
	pub fn distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		match &self.surface {
			WaterSurface::Hydro { complex } => complex.water_distance(p, terrain_height),
			WaterSurface::Flat { level } => {
				let d_xz = self.region.sdf(Vec2::new(p.x, p.z));
				let d_top = p.y - *level;
				let d_bot = terrain_height - p.y;
				d_xz.max(d_top).max(d_bot)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use jersey_terrain_stamps::CircleRegion;

	#[test]
	fn flat_slab_is_between_terrain_and_w() -> anyhow::Result<()> {
		let w = 40.0;
		let fill = WaterFill {
			region: Region2D::Circle(CircleRegion {
				center: Vec2::ZERO,
				radius: 50.0,
			}),
			surface: WaterSurface::Flat { level: w },
		};
		let h = 36.0;
		assert!(fill.wet_y_span_at(0.0, 0.0, h).is_some());
		assert!(fill.distance(Vec3::new(0.0, 38.0, 0.0), h) < 0.0);
		assert!(fill.distance(Vec3::new(0.0, w + 1.0, 0.0), h) > 0.0);
		assert!(fill.distance(Vec3::new(0.0, h - 1.0, 0.0), h) > 0.0);
		// Bank above W → dry column.
		assert!(fill.wet_y_span_at(0.0, 0.0, w + 1.75).is_none());
		// Outside region → outside.
		assert!(fill.distance(Vec3::new(80.0, 38.0, 0.0), h) > 0.0);
		Ok(())
	}
}
