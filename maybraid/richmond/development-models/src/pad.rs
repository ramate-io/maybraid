//! Constant-height pad modulation for a development cell.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use jersey_terrain_stamps::{JerseyModulation, RectRegion, Region2D, RegionAffineModulation};
use procedural_common::Bounds2;

use crate::cell::PAD_EDGE_EASE;

/// Flatten the full development cell to `height` with a short edge ease.
pub fn flatten_pad(cell: Aabb3d, height: f32) -> JerseyModulation {
	let min = Vec2::new(cell.min.x, cell.min.z);
	let max = Vec2::new(cell.max.x, cell.max.z);
	let center = (min + max) * 0.5;
	let half = (max - min) * 0.5;
	let region = Region2D::Rect(RectRegion { center, half_extents: half, round: 2.0 });
	let ease = PAD_EDGE_EASE;
	JerseyModulation::Affine(RegionAffineModulation::new(region, 0.0, height, ease, ease))
}

/// XZ bounds of a development (or terrain) cell.
pub fn cell_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::DevelopmentExtent;

	#[test]
	fn pad_center_is_constant_height() -> anyhow::Result<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let pad = flatten_pad(cell, 42.0);
		let c = Vec2::new(50.0, 50.0);
		assert!((pad.modify_elevation(10.0, c.x, c.y) - 42.0).abs() < 1e-3);
		assert!((pad.modify_elevation(90.0, c.x, c.y) - 42.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn pad_outside_cell_is_identity() -> anyhow::Result<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let pad = flatten_pad(cell, 42.0);
		let h = pad.modify_elevation(17.0, 200.0, 200.0);
		assert!((h - 17.0).abs() < 1e-3, "far sample {h}");
		Ok(())
	}
}
