//! Private roof kit tessellation (not part of the public IR).

use bevy_math::Vec3;
use scene_ref::MirrorAxis;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::panels::{PanelGeometry, PanelStyle, Rectangle, RightTriangle};
use crate::placed::Placed;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::style::RoofStyle;

/// Atomic roof kit pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoofKit {
	RightTriangle { mirror: Option<MirrorAxis> },
	Rectangle,
	DomeArc(ArcKit),
}

impl RoofGeometry {
	pub(crate) fn kit_pieces_for_style(&self, style: RoofStyle) -> Vec<Placed<RoofKit>> {
		let panel_style = PanelStyle::from(style);
		match self {
			Self::Pitch(p) => map_leaves(PanelGeometry::Quad(p.to_quad()).flatten(panel_style)),
			Self::Quad(q) => map_leaves(PanelGeometry::Quad(*q).flatten(panel_style)),
			Self::QuadPolyline(pl) => {
				map_leaves(PanelGeometry::QuadPolyline(pl.clone()).flatten(panel_style))
			}
			Self::Dome(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(RoofKit::DomeArc(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

fn map_leaves(pieces: Vec<Placed<PanelGeometry>>) -> Vec<Placed<RoofKit>> {
	pieces
		.into_iter()
		.filter_map(|p| {
			let kit = match p.geom {
				PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
					RoofKit::RightTriangle { mirror }
				}
				PanelGeometry::Rectangle(Rectangle) => RoofKit::Rectangle,
				PanelGeometry::Joint(_) => return None,
				_ => return None,
			};
			Some(Placed { geom: kit, placement: p.placement })
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::arc_kit::ArcKit;
	use crate::roofs::geometry::Pitch;
	use crate::roofs::style::RoofStyle;
	use std::f32::consts::PI;

	fn kit_pieces(g: &RoofGeometry) -> Vec<Placed<RoofKit>> {
		g.kit_pieces_for_style(RoofStyle::ShepherdsThatch)
	}

	#[test]
	fn rectangle_only_fits_tiles_to_length() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(3.0);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 6);
		assert_eq!(pieces[0].translation().x, 0.0);
		assert_eq!(pieces[0].scale(), Vec3::new(1.0, 1.0, 2.0));
		assert_eq!(pieces[2].translation().x, 1.0);
		Ok(())
	}

	#[test]
	fn tile_width_suggestion_rounds_and_stretches() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_length(2.4);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 4);
		assert!((pieces[0].scale().x - 1.2).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn left_end_shifts_rectangle_and_anchors_at_zero() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_length(2.0).with_left(0.5);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces[0].translation().x, 0.5);
		assert_eq!(pieces[0].scale().x, 0.5);
		assert_eq!(pieces[0].geom, RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) });
		assert_eq!(pieces[1].translation().x, 0.5);
		Ok(())
	}

	#[test]
	fn negative_right_uses_flipped_complement() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(1.0).with_right(-0.75);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		let end = pieces.last().expect("right end");
		assert_eq!(end.yaw(), PI);
		assert!((end.translation().x - 1.0).abs() < 1e-4);
		assert!((end.translation().z - (-2.0)).abs() < 1e-4);
		assert!((end.scale().x - 0.75).abs() < 1e-4);
		assert_eq!(end.geom, RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) });
		Ok(())
	}

	#[test]
	fn from_eave_ridge_ridge_longer_flips_both_ends() -> anyhow::Result<()> {
		let pitch = Pitch::from_eave_ridge(1.0, 2.0, 4.0, 6.0, 1.0);
		assert_eq!(pitch.length, Some(4.0));
		assert_eq!(pitch.left, Some(-1.0));
		assert_eq!(pitch.right, Some(-1.0));
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
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
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 2);
		Ok(())
	}

	#[test]
	fn dome_forty_five_is_three_fifteens() -> anyhow::Result<()> {
		let pieces = kit_pieces(&RoofGeometry::dome(45.0));
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|p| p.geom == RoofKit::DomeArc(ArcKit::D15)));
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
		let via_pitch = kit_pieces(&RoofGeometry::pitch(pitch));
		let via_quad = kit_pieces(&RoofGeometry::quad(pitch.to_quad()));
		assert_eq!(via_pitch.len(), via_quad.len());
		Ok(())
	}
}
