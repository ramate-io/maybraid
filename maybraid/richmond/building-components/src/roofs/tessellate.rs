//! Private roof kit tessellation (not part of the public IR).
//!
//! Kits are placed in **flat** roof-plane space. The unit right-triangle kit is
//! origin-anchored with \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\)
//! (see `unit_right_triangle.glb`). [`crate::roofs::node::RoofNode`] applies
//! pitch about local +X after kit poses.
//!
//! Rectangular half gables tile **along +X** (horizontal length). Pitch about +X
//! then lifts **Z** into the slope so the rectangle stays a pitched wall along X.
//! The complementary triangle of each unit square is yaw-π with its origin at the
//! far corner \((x+1, 0, -1)\), matching the origin-anchored footprint.

use bevy_math::Vec3;
use std::f32::consts::PI;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::placed::Placed;
use crate::roofs::geometry::RoofGeometry;

/// Atomic roof kit pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoofKit {
	/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
	RightTriangle,
	/// Dome arc kit (empty leaf scenes until bespoke GLBs exist).
	DomeArc(ArcKit),
}

impl RoofGeometry {
	/// Expand continuous geometry into placed kit pieces (flat roof-plane space).
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<RoofKit>> {
		match self {
			Self::RectangularHalfGable(g) => rectangular_half_gable_pieces(g.length_units),
			Self::RectangularIntersectingHalfGable(g) => {
				rectangular_intersecting_half_gable_pieces(g.length_units, g.end_triangle_scale)
			}
			Self::HalfTriangularHip(_) => vec![Placed::at_origin(RoofKit::RightTriangle)],
			Self::HalfTrapezoidalHip(g) => half_trapezoidal_hip_pieces(g.edge_units),
			Self::Dome(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(RoofKit::DomeArc(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

/// Two mirrored right triangles fill the unit square \(X \in [x, x+1]\), \(Z \in [-1, 0]\).
///
/// The kit is origin-anchored into \(+X/-Z\). After yaw \(\pi\), the same footprint
/// lands in \(-X/+Z\) relative to its transform origin, so that origin must sit at
/// the far corner \((x+1, 0, -1)\) to cover the complementary half.
fn unit_square_pair_along_x(x: f32) -> [Placed<RoofKit>; 2] {
	[
		Placed::new(RoofKit::RightTriangle, Vec3::new(x, 0.0, 0.0), 0.0),
		Placed::new(RoofKit::RightTriangle, Vec3::new(x + 1.0, 0.0, -1.0), PI),
	]
}

fn rectangular_half_gable_pieces(length_units: u32) -> Vec<Placed<RoofKit>> {
	let n = length_units.max(1);
	let mut out = Vec::with_capacity((n * 2) as usize);
	for i in 0..n {
		out.extend(unit_square_pair_along_x(i as f32));
	}
	out
}

/// Same as rectangular half gable, but the far-end bottom (mirrored) triangle is scaled.
fn rectangular_intersecting_half_gable_pieces(
	length_units: u32,
	end_triangle_scale: f32,
) -> Vec<Placed<RoofKit>> {
	let n = length_units.max(1);
	let mut out = Vec::with_capacity((n * 2) as usize);
	for i in 0..n {
		let x = i as f32;
		out.push(Placed::new(
			RoofKit::RightTriangle,
			Vec3::new(x, 0.0, 0.0),
			0.0,
		));
		let bottom = Placed::new(
			RoofKit::RightTriangle,
			Vec3::new(x + 1.0, 0.0, -1.0),
			PI,
		);
		if i + 1 == n {
			out.push(bottom.with_scale(Vec3::new(
				end_triangle_scale,
				1.0,
				end_triangle_scale,
			)));
		} else {
			out.push(bottom);
		}
	}
	out
}

/// Base triangle plus `edge_units` alternating companions that form a roofline edge.
fn half_trapezoidal_hip_pieces(edge_units: u32) -> Vec<Placed<RoofKit>> {
	let edge = edge_units.max(1);
	let mut out = Vec::with_capacity((1 + edge) as usize);
	out.push(Placed::at_origin(RoofKit::RightTriangle));
	for i in 0..edge {
		let x = i as f32;
		if i % 2 == 0 {
			out.push(Placed::new(
				RoofKit::RightTriangle,
				Vec3::new(x + 1.0, 0.0, -1.0),
				PI,
			));
		} else {
			out.push(Placed::new(
				RoofKit::RightTriangle,
				Vec3::new(x + 1.0, 0.0, 0.0),
				0.0,
			));
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::arc_kit::ArcKit;

	#[test]
	fn rectangular_half_gable_two_triangles_per_unit() -> anyhow::Result<()> {
		let pieces = RoofGeometry::rectangular_half_gable(3, 30.0).kit_pieces();
		assert_eq!(pieces.len(), 6);
		assert!(pieces.iter().all(|p| p.geom == RoofKit::RightTriangle));
		Ok(())
	}

	#[test]
	fn rectangular_half_gable_tiles_along_x_with_origin_anchored_mirror() -> anyhow::Result<()> {
		let pieces = RoofGeometry::rectangular_half_gable(3, 30.0).kit_pieces();
		// Kit footprint is +X/−Z from the origin; complementary half sits at (x+1, 0, −1).
		assert_eq!(pieces[0].translation(), Vec3::new(0.0, 0.0, 0.0));
		assert_eq!(pieces[1].translation(), Vec3::new(1.0, 0.0, -1.0));
		assert_eq!(pieces[1].yaw(), PI);
		assert_eq!(pieces[2].translation(), Vec3::new(1.0, 0.0, 0.0));
		assert_eq!(pieces[4].translation(), Vec3::new(2.0, 0.0, 0.0));
		Ok(())
	}

	#[test]
	fn rectangular_half_gable_clamps_zero_length() -> anyhow::Result<()> {
		let pieces = RoofGeometry::rectangular_half_gable(0, 15.0).kit_pieces();
		assert_eq!(pieces.len(), 2);
		Ok(())
	}

	#[test]
	fn intersecting_end_triangle_carries_scale() -> anyhow::Result<()> {
		let pieces =
			RoofGeometry::rectangular_intersecting_half_gable(2, 25.0, 0.5).kit_pieces();
		assert_eq!(pieces.len(), 4);
		let end = pieces.last().expect("end triangle");
		assert_eq!(end.scale(), Vec3::new(0.5, 1.0, 0.5));
		assert!(pieces[..3].iter().all(|p| p.scale() == Vec3::ONE));
		assert_eq!(end.translation(), Vec3::new(2.0, 0.0, -1.0));
		Ok(())
	}

	#[test]
	fn half_triangular_hip_is_one_triangle() -> anyhow::Result<()> {
		let pieces = RoofGeometry::half_triangular_hip(40.0).kit_pieces();
		assert_eq!(pieces.len(), 1);
		assert_eq!(pieces[0].geom, RoofKit::RightTriangle);
		Ok(())
	}

	#[test]
	fn half_trapezoidal_hip_base_plus_edge() -> anyhow::Result<()> {
		let pieces = RoofGeometry::half_trapezoidal_hip(35.0, 2).kit_pieces();
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|p| p.geom == RoofKit::RightTriangle));
		Ok(())
	}

	#[test]
	fn dome_forty_five_is_three_fifteens() -> anyhow::Result<()> {
		let pieces = RoofGeometry::dome(45.0).kit_pieces();
		assert_eq!(pieces.len(), 3);
		assert!(pieces
			.iter()
			.all(|p| p.geom == RoofKit::DomeArc(ArcKit::D15)));
		Ok(())
	}

	#[test]
	fn dome_one_eighty_is_single_piece() -> anyhow::Result<()> {
		let pieces = RoofGeometry::dome(180.0).kit_pieces();
		assert_eq!(pieces.len(), 1);
		assert_eq!(pieces[0].geom, RoofKit::DomeArc(ArcKit::D180));
		assert_eq!(pieces[0].yaw(), 0.0);
		Ok(())
	}

	#[test]
	fn dome_pitch_is_zero() -> anyhow::Result<()> {
		assert_eq!(RoofGeometry::dome(90.0).pitch_degrees(), 0.0);
		Ok(())
	}
}
