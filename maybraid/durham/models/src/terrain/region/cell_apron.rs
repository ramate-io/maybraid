//! Owning-cell apron: force modulation identity at/outside a macro tile.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

/// XZ bounds of the cell that owns a modulation, plus an interior fade width.
///
/// `interior_weight` is 1 deep inside the cell and 0 on/outside the boundary so
/// stamp softmask cannot affect neighboring tiles.
#[derive(Debug, Clone)]
pub struct CellApron {
	pub min: Vec2,
	pub max: Vec2,
	pub apron: f32,
}

impl CellApron {
	pub fn from_aabb(cell: Aabb3d, apron: f32) -> Self {
		Self {
			min: Vec2::new(cell.min.x, cell.min.z),
			max: Vec2::new(cell.max.x, cell.max.z),
			apron: apron.max(1e-3),
		}
	}

	#[inline(always)]
	fn smoothstep(t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);
		t * t * (3.0 - 2.0 * t)
	}

	/// Modulation strength factor: `1` inside the apron band, `0` at/outside the face.
	#[inline(always)]
	pub fn interior_weight(&self, p: Vec2) -> f32 {
		let center = (self.min + self.max) * 0.5;
		let half_extents = (self.max - self.min) * 0.5;
		let q = (p - center).abs() - half_extents;
		let d = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
		if d >= 0.0 {
			0.0
		} else if d <= -self.apron {
			1.0
		} else {
			// d in (-apron, 0): fade from 1 at -apron to 0 at the face.
			Self::smoothstep((-d) / self.apron)
		}
	}
}
