//! Private roof kit tessellation (not part of the public IR).
//!
//! The unit right-triangle kit is origin-anchored with \(X \in [0, 1]\),
//! \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\) (`panels/unit_right_triangle.glb`).
//! [`crate::roofs::node::RoofNode`] applies pitch about local +X after kit poses.
//!
//! [`Pitch`](crate::roofs::geometry::Pitch) layouts are lower-left anchored via shared
//! [`crate::panels::Quad`] tessellation.

use bevy_math::Vec3;
use scene_ref::MirrorAxis;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::panels::{
	PanelGeom, QuadPolyline, Rectangle, RightTriangle, TessellatePolicy,
};
use crate::placed::Placed;
use crate::roofs::geometry::RoofGeometry;

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
	/// Rectangular panel tile (styles without a rectangle kit should prefer dual triangles).
	Rectangle,
	/// Dome arc kit (empty leaf scenes until bespoke GLBs exist).
	DomeArc(ArcKit),
}

impl RoofGeometry {
	/// Expand continuous geometry into placed kit pieces (flat roof-plane space).
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<RoofKit>> {
		match self {
			Self::Pitch(p) => map_panel_atoms(p.to_quad().decompose(TessellatePolicy::DUAL_TRIANGLES)),
			Self::Quad(q) => map_panel_atoms(q.decompose(self.tessellate_policy())),
			Self::QuadPolyline(pl) => expand_quad_polyline(pl, self.tessellate_policy()),
			Self::Dome(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(RoofKit::DomeArc(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

fn expand_quad_polyline(pl: &QuadPolyline, policy: TessellatePolicy) -> Vec<Placed<RoofKit>> {
	let mut out = Vec::new();
	for piece in pl.decompose() {
		match piece.geom {
			PanelGeom::Quad(q) => {
				for child in q.decompose(policy) {
					out.push(Placed {
						geom: panel_atom_to_roof(child.geom),
						placement: piece.placement.compose_child(child.placement),
					});
				}
			}
			PanelGeom::Joint(_) => {
				// Joint kits are partition-oriented; roofs omit them in v1.
			}
			other => {
				out.push(Placed {
					geom: panel_atom_to_roof(other),
					placement: piece.placement,
				});
			}
		}
	}
	out
}

fn map_panel_atoms(pieces: Vec<Placed<PanelGeom>>) -> Vec<Placed<RoofKit>> {
	pieces
		.into_iter()
		.filter_map(|p| {
			let kit = match p.geom {
				PanelGeom::RightTriangle(RightTriangle { mirror }) => {
					RoofKit::RightTriangle { mirror }
				}
				PanelGeom::Rectangle(Rectangle) => RoofKit::Rectangle,
				_ => return None,
			};
			Some(Placed {
				geom: kit,
				placement: p.placement,
			})
		})
		.collect()
}

fn panel_atom_to_roof(geom: PanelGeom) -> RoofKit {
	match geom {
		PanelGeom::RightTriangle(RightTriangle { mirror }) => RoofKit::RightTriangle { mirror },
		PanelGeom::Rectangle(_) => RoofKit::Rectangle,
		_ => RoofKit::Rectangle,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::arc_kit::ArcKit;
	use crate::roofs::geometry::Pitch;
	use std::f32::consts::PI;

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

	#[test]
	fn quad_geometry_matches_pitch() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(2.0).with_left(0.5);
		let via_pitch = RoofGeometry::pitch(pitch).kit_pieces();
		let via_quad = RoofGeometry::quad(pitch.to_quad()).kit_pieces();
		assert_eq!(via_pitch.len(), via_quad.len());
		Ok(())
	}
}
