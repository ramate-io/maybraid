//! Normalized stair geometry components (tread kit pieces).

use std::f32::consts::TAU;

use bevy_math::Vec3;

use crate::placed::{IntoGeometryComponents, Placed};
use crate::stairs::geometry::{SpiralStair, Stair, StraightStair};

/// Stair kit half-extent (\(X = Y = Z \in [-1, 1]\)).
pub const TREAD_HALF_EXTENT: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StairComponent {
	/// Single rough-stone tread cube (`rough_stonework_tread_001`).
	Tread,
	Spiral,
	Straight,
}

impl IntoGeometryComponents for Stair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		match self {
			Self::Spiral(g) => g.into_geometry_components(),
			Self::Straight(g) => g.into_geometry_components(),
		}
	}
}

impl IntoGeometryComponents for SpiralStair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		let tops = self.effective_tread_tops();
		if tops.is_empty() {
			return Vec::new();
		}
		let n = tops.len() as f32;
		let yaw_step = self.turns * TAU / n;
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
				let translation = Vec3::new(c * self.radius, top - 0.5 * rise, -s * self.radius);
				let scale = Vec3::new(
					self.tread_depth / half,
					rise / half,
					self.tread_width / half,
				);
				Placed::new(StairComponent::Tread, translation, yaw + std::f32::consts::FRAC_PI_2)
					.with_scale(scale)
			})
			.collect()
	}
}

impl IntoGeometryComponents for StraightStair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		vec![Placed::at_origin(StairComponent::Straight)]
	}
}
