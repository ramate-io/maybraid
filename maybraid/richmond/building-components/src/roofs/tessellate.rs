//! Private roof kit tessellation (not part of the public IR).
//!
//! The unit right-triangle kit is origin-anchored with \(X \in [0, 1]\),
//! \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\) (`panels/unit_right_triangle.glb`).
//! [`crate::roofs::node::RoofNode`] applies pitch about local +X after kit poses.
//!
//! [`Pitch`](crate::roofs::geometry::Pitch) layouts are lower-left anchored: optional
//! left end triangle, then the rectangular body, then optional right end triangle.
//! Eave sits at \(Z = 0\); run scales toward \(Z = -\texttt{run}\).

use bevy_math::Vec3;
use scene_ref::MirrorAxis;
use std::f32::consts::PI;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::placed::{Placement, Placed};
use crate::roofs::geometry::{Pitch, RoofGeometry};

/// Atomic roof kit pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoofKit {
	/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
	///
	/// When `mirror` is set, the GLB is rebuilt via [`scene_ref::SceneRef`] mirroring (positive
	/// Transform scale) instead of a negative `scale.x`.
	RightTriangle {
		mirror: Option<MirrorAxis>,
	},
	/// Dome arc kit (empty leaf scenes until bespoke GLBs exist).
	DomeArc(ArcKit),
}

impl RoofGeometry {
	/// Expand continuous geometry into placed kit pieces (flat roof-plane space).
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<RoofKit>> {
		match self {
			Self::Pitch(p) => pitch_pieces(*p),
			Self::Dome(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(RoofKit::DomeArc(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

fn tile_scale(width: f32, run: f32) -> Vec3 {
	Vec3::new(width.max(1e-4), 1.0, run.max(1e-4))
}

fn fitted_tile_count(length: f32, tile_width: f32) -> u32 {
	let tw = tile_width.max(1e-4);
	((length / tw).round() as i32).max(1) as u32
}

/// Two complementary right triangles fill one tile square along +X.
fn unit_square_pair(x: f32, width: f32, run: f32) -> [Placed<RoofKit>; 2] {
	let scale = tile_scale(width, run);
	[
		Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x, 0.0, 0.0), 0.0).with_scale(scale),
		),
		Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x + width, 0.0, -run), PI).with_scale(scale),
		),
	]
}

/// Which end of the pitch rectangle an end-cap attaches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndSide {
	Left,
	Right,
}

/// End triangle. Positive base → eave-long (upright); negative → ridge-long (flipped).
///
/// Left upright and right ridge-long use [`MirrorAxis::X`] with positive scale so
/// materials stay single-sided.
fn end_triangle(side: EndSide, x_min: f32, base: f32, run: f32) -> Placed<RoofKit> {
	let width = base.abs().max(1e-4);
	let run = run.max(1e-4);
	let scale = Vec3::new(width, 1.0, run);
	match (side, base >= 0.0) {
		// Left eave-long: right angle on the rectangle edge, mirrored on X.
		(EndSide::Left, true) => Placed::with_placement(
			RoofKit::RightTriangle {
				mirror: Some(MirrorAxis::X),
			},
			Placement::new(Vec3::new(x_min + width, 0.0, 0.0), 0.0).with_scale(scale),
		),
		// Left ridge-long: complement at the rectangle's ridge corner.
		(EndSide::Left, false) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x_min + width, 0.0, -run), PI).with_scale(scale),
		),
		// Right eave-long: primary at the rectangle edge.
		(EndSide::Right, true) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x_min, 0.0, 0.0), 0.0).with_scale(scale),
		),
		// Right ridge-long: mirrored complement at the rectangle's ridge corner.
		(EndSide::Right, false) => Placed::with_placement(
			RoofKit::RightTriangle {
				mirror: Some(MirrorAxis::X),
			},
			Placement::new(Vec3::new(x_min, 0.0, -run), PI).with_scale(scale),
		),
	}
}

fn pitch_pieces(pitch: Pitch) -> Vec<Placed<RoofKit>> {
	let run = pitch.run.max(0.0);
	let mut out = Vec::new();

	let left_w = pitch.left.map(|b| b.abs()).unwrap_or(0.0);
	if let Some(base) = pitch.left {
		if base.abs() > 1e-6 {
			out.push(end_triangle(EndSide::Left, 0.0, base, run));
		}
	}

	let rect_x0 = left_w;
	if let Some(length) = pitch.length {
		if length > 1e-6 {
			let n = fitted_tile_count(length, pitch.tile_width);
			let width = length / n as f32;
			for i in 0..n {
				out.extend(unit_square_pair(
					rect_x0 + i as f32 * width,
					width,
					run,
				));
			}
		}
	}

	if let Some(base) = pitch.right {
		if base.abs() > 1e-6 {
			let x_min = rect_x0 + pitch.length.unwrap_or(0.0);
			out.push(end_triangle(EndSide::Right, x_min, base, run));
		}
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::arc_kit::ArcKit;
	use crate::roofs::geometry::Pitch;

	#[test]
	fn rectangle_only_fits_tiles_to_length() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(3.0);
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		// 3 tiles × 2 triangles
		assert_eq!(pieces.len(), 6);
		assert_eq!(pieces[0].translation().x, 0.0);
		assert_eq!(pieces[0].scale(), Vec3::new(1.0, 1.0, 2.0));
		assert_eq!(pieces[2].translation().x, 1.0);
		Ok(())
	}

	#[test]
	fn tile_width_suggestion_rounds_and_stretches() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_length(2.4);
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		// round(2.4/1)=2 tiles → width 1.2
		assert_eq!(pieces.len(), 4);
		assert!((pieces[0].scale().x - 1.2).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn left_end_shifts_rectangle_and_anchors_at_zero() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0)
			.with_length(2.0)
			.with_left(0.5);
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		// Left upright: origin on the rectangle edge, positive scale + X mirror.
		assert_eq!(pieces[0].translation().x, 0.5);
		assert_eq!(pieces[0].scale().x, 0.5);
		assert_eq!(
			pieces[0].geom,
			RoofKit::RightTriangle {
				mirror: Some(MirrorAxis::X),
			}
		);
		// Rectangle starts after left base.
		assert_eq!(pieces[1].translation().x, 0.5);
		Ok(())
	}

	#[test]
	fn negative_right_uses_flipped_complement() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0)
			.with_length(1.0)
			.with_right(-0.75);
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		let end = pieces.last().expect("right end");
		assert_eq!(end.yaw(), PI);
		// Mirrored ridge-long: origin at rect ridge corner, positive scale + X mirror.
		assert!((end.translation().x - 1.0).abs() < 1e-4);
		assert!((end.translation().z - (-2.0)).abs() < 1e-4);
		assert!((end.scale().x - 0.75).abs() < 1e-4);
		assert_eq!(
			end.geom,
			RoofKit::RightTriangle {
				mirror: Some(MirrorAxis::X),
			}
		);
		Ok(())
	}

	#[test]
	fn from_eave_ridge_ridge_longer_flips_both_ends() -> anyhow::Result<()> {
		let pitch = Pitch::from_eave_ridge(1.0, 2.0, 4.0, 6.0, 1.0);
		assert_eq!(pitch.length, Some(4.0));
		assert_eq!(pitch.left, Some(-1.0));
		assert_eq!(pitch.right, Some(-1.0));
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		// left flipped + 4 tiles×2 + right flipped
		assert_eq!(pieces.len(), 1 + 8 + 1);
		assert_eq!(pieces[0].yaw(), PI);
		assert_eq!(pieces.last().expect("right").yaw(), PI);
		Ok(())
	}

	#[test]
	fn from_eave_ridge_eave_longer_upright_ends() -> anyhow::Result<()> {
		let pitch = Pitch::from_eave_ridge(1.0, 2.0, 6.0, 4.0, 1.0);
		assert_eq!(pitch.length, Some(4.0));
		assert_eq!(pitch.left, Some(1.0));
		assert_eq!(pitch.right, Some(1.0));
		Ok(())
	}

	#[test]
	fn length_none_omits_rectangle() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_left(1.0).with_right(1.0);
		let pieces = RoofGeometry::pitch(pitch).kit_pieces();
		assert_eq!(pieces.len(), 2);
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
	fn dome_pitch_is_zero() -> anyhow::Result<()> {
		assert_eq!(RoofGeometry::dome(90.0).pitch_degrees(), 0.0);
		Ok(())
	}

	#[test]
	fn pitch_radians_from_rise_run() -> anyhow::Result<()> {
		let p = Pitch::new(1.0, 1.0, 1.0).with_length(1.0);
		assert!((p.pitch_radians() - std::f32::consts::FRAC_PI_4).abs() < 1e-4);
		Ok(())
	}
}
