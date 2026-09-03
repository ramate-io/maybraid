//! Union-first building-pad primitives + broadphase helpers.
//!
//! Skirt ease lives on [`PadParams`]. Terrain flatten / ease blend lives on
//! [`crate::pad::node::PadNode`]; complexes gather intersecting nodes.
//!
//! Rectangular flatten only for now; grading elevation comes later.

pub mod complex;
pub mod elevation;
pub mod footprint;
pub mod node;

pub use complex::PadComplex;
pub use elevation::PadElevation;
pub use footprint::{PadFootprint, PadRect};
pub use node::{PadNode, PadStage};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use procedural_common::Bounds2;

use crate::cell::{PAD_BERM, PAD_EDGE_EASE, PAD_ROUND};

/// Ease / berm parameters for one pad node (grading knobs later).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadParams {
	/// Extra flatten outside the building footprint (metres).
	pub berm: f32,
	/// Ease / apron width from flatten support out to identity terrain.
	pub ease: f32,
	/// Rounded-rect corner radius on the flatten footprint.
	pub round: f32,
}

impl Default for PadParams {
	fn default() -> Self {
		Self { berm: PAD_BERM, ease: PAD_EDGE_EASE, round: PAD_ROUND }
	}
}

/// One pad leaf: footprint + local elevation field.
#[derive(Debug, Clone)]
pub struct PadPrimitive {
	pub footprint: PadFootprint,
	pub elevation: PadElevation,
	/// Extra AABB pad for broadphase / ease support (world units).
	pub influence_pad: f32,
}

impl PadPrimitive {
	pub fn aabb(&self) -> (Vec2, Vec2) {
		self.footprint.aabb()
	}

	pub fn phi(&self, p: Vec2) -> f32 {
		self.footprint.sdf(p)
	}
}

/// XZ bounds of a development (or terrain) cell.
pub fn cell_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

/// Cell-centered XZ sample used as the building / pad origin.
pub fn cell_center_xz(cell: Aabb3d) -> Vec2 {
	Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::{BUILDING_INSET, PAD_BERM, PAD_EDGE_EASE};
	use procedural_common::Bounds2;
	use std::f32::consts::FRAC_PI_4;

	fn skirt(half: Vec2, yaw: f32, height: f32) -> PadComplex {
		PadComplex::building_skirt(
			Bounds2::from_xz(-80.0, -80.0, 80.0, 80.0),
			Vec2::ZERO,
			half,
			yaw,
			height,
			PadParams::default(),
		)
	}

	#[test]
	fn berm_plus_ease_matches_building_inset() {
		assert!((BUILDING_INSET - (PAD_BERM + PAD_EDGE_EASE)).abs() < 1e-6);
	}

	#[test]
	fn flatten_center_is_constant_height() {
		let pad = skirt(Vec2::new(20.0, 12.0), 0.0, 42.0);
		assert!((pad.modify_elevation(10.0, 0.0, 0.0) - 42.0).abs() < 1e-3);
		assert!((pad.modify_elevation(90.0, 0.0, 0.0) - 42.0).abs() < 1e-3);
		assert_eq!(pad.classification_at(0.0, 0.0), Some(PadStage::Flatten));
	}

	#[test]
	fn far_sample_is_identity() {
		let pad = skirt(Vec2::new(20.0, 12.0), 0.0, 42.0);
		let h = pad.modify_elevation(17.0, 70.0, 70.0);
		assert!((h - 17.0).abs() < 1e-3, "far sample {h}");
	}

	#[test]
	fn ease_band_sits_between_pad_and_terrain() {
		let half_x = 20.0;
		let pad = skirt(Vec2::new(half_x, 12.0), 0.0, 40.0);
		let berm = PadParams::default().berm;
		let x_ease = half_x + berm + 3.0;
		let h = pad.modify_elevation(10.0, x_ease, 0.0);
		assert_eq!(pad.classification_at(x_ease, 0.0), Some(PadStage::Ease));
		assert!(h > 10.0 + 1.0, "ease should lift toward pad: {h}");
		assert!(h < 40.0 - 1.0, "ease should not be full flatten: {h}");
	}

	#[test]
	fn yawed_flatten_follows_building_x() {
		let half = Vec2::new(18.0, 8.0);
		let pad = skirt(half, FRAC_PI_4, 30.0);
		let (s, c) = FRAC_PI_4.sin_cos();
		let local_x = 10.0;
		let p = Vec2::new(c * local_x, s * local_x);
		assert_eq!(pad.classification_at(p.x, p.y), Some(PadStage::Flatten));
		assert!((pad.modify_elevation(1.0, p.x, p.y) - 30.0).abs() < 1e-3);
		let away = Vec2::new(-s * 40.0, c * 40.0);
		let h = pad.modify_elevation(7.0, away.x, away.y);
		assert!((h - 7.0).abs() < 1e-3, "lateral far field {h}");
	}

	#[test]
	fn overlapping_flatten_prefers_tightest_terrace() {
		let outer = PadNode::rectangular_flatten(
			Vec2::ZERO,
			Vec2::new(20.0, 20.0),
			0.0,
			10.0,
			PadParams { berm: 0.0, ease: 4.0, round: 0.0 },
		);
		let inner = PadNode::rectangular_flatten(
			Vec2::ZERO,
			Vec2::new(6.0, 6.0),
			0.0,
			22.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		let complex =
			PadComplex::from_nodes(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), vec![outer, inner]);
		assert!((complex.modify_elevation(0.0, 0.0, 0.0) - 22.0).abs() < 1e-3);
	}
}
