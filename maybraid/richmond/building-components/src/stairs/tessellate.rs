//! Private stair kit tessellation (not part of the public IR).

use std::f32::consts::TAU;

use bevy_math::Vec3;

use crate::placed::{Placement, Placed};
use crate::stairs::geometry::{SpiralStair, StairGeometry};

/// Stair kit half-extent (\(X = Y = Z \in [-1, 1]\)).
pub(crate) const TREAD_HALF_EXTENT: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Spiral reserved for non-tread spiral placeholders
pub(crate) enum StairKit {
	/// Single rough-stone tread cube (`rough_stonework_tread_001`).
	Tread,
	Spiral,
	Straight,
}

impl StairGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<StairKit>> {
		match self {
			Self::Spiral(g) => spiral_kits(g),
			Self::Straight(_) => vec![Placed::at_origin(StairKit::Straight)],
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

fn spiral_kits(g: &SpiralStair) -> Vec<Placed<StairKit>> {
	let tops = g.effective_tread_tops();
	if tops.is_empty() {
		return Vec::new();
	}
	let n = tops.len() as f32;
	let yaw_step = g.turns * TAU / n;
	let half = 2.0 * TREAD_HALF_EXTENT;
	let mut prev_top = 0.0_f32;

	tops.into_iter()
		.enumerate()
		.map(|(i, top)| {
			let rise = (top - prev_top).max(1e-4);
			prev_top = top;
			let yaw = i as f32 * yaw_step;
			let (s, c) = yaw.sin_cos();
			// Centerline on the circle; yaw so local +X is tangential (ascent).
			// Kit: left face = −Z; bleed support extends toward −X.
			let translation = Vec3::new(c * g.radius, top - 0.5 * rise, -s * g.radius);
			let scale = Vec3::new(g.tread_depth / half, rise / half, g.tread_width / half);
			Placed::new(StairKit::Tread, translation, yaw + std::f32::consts::FRAC_PI_2)
				.with_scale(scale)
		})
		.collect()
}

