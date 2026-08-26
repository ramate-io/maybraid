//! Private stair kit tessellation (not part of the public IR).

use bevy_math::Vec3;

use crate::placed::{Placed, Placement};
use crate::stairs::geometry::{StraightStair, StairGeometry};

/// Stair kit half-extent (\(X = Y = Z \in [-1, 1]\)).
pub(crate) const TREAD_HALF_EXTENT: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StairKit {
	/// Single rough-stone tread cube (`rough_stonework_tread_001`).
	Tread,
}

impl StairGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<StairKit>> {
		match self {
			Self::Straight(g) => straight_kits(g),
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<StairKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

fn straight_kits(g: &StraightStair) -> Vec<Placed<StairKit>> {
	let tops = g.effective_tread_tops();
	if tops.is_empty() {
		return Vec::new();
	}
	let going = g.going_per_tread();
	let half = 2.0 * TREAD_HALF_EXTENT;
	let mut prev_top = 0.0_f32;

	tops.into_iter()
		.enumerate()
		.map(|(i, top)| {
			let rise = (top - prev_top).max(1e-4);
			prev_top = top;
			// First tread centered on the origin in XZ; later treads step +X.
			let translation = Vec3::new(i as f32 * going, top - 0.5 * rise, 0.0);
			let scale = Vec3::new(going / half, rise / half, g.width / half);
			Placed::new(StairKit::Tread, translation, 0.0).with_scale(scale)
		})
		.collect()
}
