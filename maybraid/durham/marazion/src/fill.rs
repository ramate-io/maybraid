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
//! (plus softmask fade). That matches terrain-style meshing on the shared
//! origin-cell cascade lattice: marching cubes resolves the free surface on the
//! same tall Y grid as terrain; subterranean volume is intentional and fine.
//! Islands / beds above \(W\) stay dry because the undercut gate fails.
//!
//! Lakes use a flat \(W\); streams use a piecewise grade along a polyline.

use bevy_math::{Vec2, Vec3};
use jersey_terrain_stamps::{
	closest_on_polyline, grade_along_polyline, soft_voronoi_weights, Region2D, RegionNoise,
};

/// Large positive distance used when a sample is outside the softmask boundary
/// or the column is dry (\(W \le h_{\mathrm{eff}}\)).
const OUTSIDE_FILL_DISTANCE: f32 = 1.0e6;

/// One graded corridor contributing to an owned multi-path water surface.
#[derive(Debug, Clone)]
pub struct WaterGradePart {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub node_blend: f32,
}

/// Water surface elevation model decided by the stamp.
#[derive(Debug, Clone)]
pub enum WaterSurface {
	/// Constant lake (or pool) surface.
	Flat { level: f32 },
	/// Piecewise grade along a polyline: lerp between per-node levels, with
	/// inbound/outbound pitch blending within [`Self::Graded::node_blend`] of vertices.
	Graded {
		path: Vec<Vec2>,
		/// Per-vertex water elevations (`len` should match `path`).
		levels: Vec<f32>,
		/// Path-distance blend radius for pitch mixing at nodes (world units).
		node_blend: f32,
	},
	/// Nearest-path owned grade across several corridors (one \(W\) field).
	OwnedGraded {
		parts: Vec<WaterGradePart>,
		/// Soft-voronoi sharpness; high ≈ hard nearest-path ownership.
		ownership_gamma: f32,
	},
}

impl WaterSurface {
	/// Surface elevation at a horizontal sample.
	pub fn level_at(&self, x: f32, z: f32) -> f32 {
		match self {
			Self::Flat { level } => *level,
			Self::Graded {
				path,
				levels,
				node_blend,
			} => grade_along_polyline(path, levels, Vec2::new(x, z), *node_blend),
			Self::OwnedGraded {
				parts,
				ownership_gamma,
			} => owned_grade_at(parts, *ownership_gamma, Vec2::new(x, z)),
		}
	}
}

fn owned_grade_at(parts: &[WaterGradePart], ownership_gamma: f32, p: Vec2) -> f32 {
	if parts.is_empty() {
		return 0.0;
	}
	if parts.len() == 1 {
		return grade_along_polyline(&parts[0].path, &parts[0].levels, p, parts[0].node_blend);
	}
	let weights = soft_voronoi_weights(
		parts
			.iter()
			.map(|part| closest_on_polyline(&part.path, p).distance),
		ownership_gamma,
	);
	let mut num = 0.0;
	let mut den = 0.0;
	for (i, part) in parts.iter().enumerate() {
		let w = weights.get(i).copied().unwrap_or(0.0);
		if w <= 1e-8 {
			continue;
		}
		num += w * grade_along_polyline(&part.path, &part.levels, p, part.node_blend);
		den += w;
	}
	if den <= 1e-8 {
		grade_along_polyline(&parts[0].path, &parts[0].levels, p, parts[0].node_blend)
	} else {
		num / den
	}
}

/// Stamp-owned water volume (Lake / Stream).
///
/// Surface, softmask, and terrain undercut are decided by the stamp;
/// [`Self::distance`] evaluates those products against a terrain height sample.
#[derive(Debug, Clone)]
pub struct WaterFill {
	/// Horizontal footprint (bowl / channel + bleed).
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
	/// Softmask weight in `[0, 1]` (`0` = fully inside / wet, `1` = outside / dry).
	pub fn softmask_at(&self, x: f32, z: f32) -> f32 {
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
	use jersey_terrain_stamps::{CircleRegion, PolylineRegion};

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

	#[test]
	fn graded_half_space_follows_polyline_surface() -> anyhow::Result<()> {
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)];
		let levels = vec![50.0, 42.0];
		let fill = WaterFill {
			region: Region2D::Polyline(PolylineRegion::new(path.clone(), 6.0)),
			inner_radius: 0.0,
			outer_radius: 2.0,
			noise: None,
			surface: WaterSurface::Graded {
				path,
				levels,
				node_blend: 0.0,
			},
			terrain_undercut: 2.0,
		};
		let mid_w = fill.surface_level_at(20.0, 0.0);
		assert!((mid_w - 46.0).abs() < 1e-3);
		let bed = mid_w - 3.0;
		assert!(fill.column_is_wet(20.0, 0.0, bed));
		assert!(fill.distance(Vec3::new(20.0, mid_w - 500.0, 0.0), bed) < 0.0);
		assert!(fill.distance(Vec3::new(20.0, mid_w + 1.0, 0.0), bed) > 0.0);
		Ok(())
	}

	#[test]
	fn owned_graded_picks_nearest_corridor() -> anyhow::Result<()> {
		let surface = WaterSurface::OwnedGraded {
			parts: vec![
				WaterGradePart {
					path: vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)],
					levels: vec![50.0, 50.0],
					node_blend: 0.0,
				},
				WaterGradePart {
					path: vec![Vec2::new(0.0, 20.0), Vec2::new(40.0, 20.0)],
					levels: vec![30.0, 30.0],
					node_blend: 0.0,
				},
			],
			ownership_gamma: 8.0,
		};
		assert!((surface.level_at(20.0, 0.0) - 50.0).abs() < 0.5);
		assert!((surface.level_at(20.0, 20.0) - 30.0).abs() < 0.5);
		Ok(())
	}
}
