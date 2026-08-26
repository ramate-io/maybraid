//! Upper landing: parallelogram flush with a [`TreadEnd`], along the opening rim.

use richmond_building_components::panels::PanelStyle;

use crate::connecting::geom::EPS;
use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::{StairwellFlight, TreadEnd};

use super::opening::{at_y, StairwellOpening};
use super::{MIN_SLAB_M, RUN_IN_M};

impl TreadEnd {
	/// Thin slab starting on this leading edge, extruded along the nearby rim.
	pub fn landing_slab(
		self,
		opening: StairwellOpening,
		style: PanelStyle,
		thickness: f32,
	) -> Option<QuadPanel> {
		let rim_side = if opening.rim_distance(self.leading_inner)
			< opening.rim_distance(self.leading_outer)
		{
			self.leading_inner
		} else {
			self.leading_outer
		};
		let end = self.prefer_outer_near(rim_side);
		let (edge, a0_xz) = opening.nearest_rim(end.leading_outer)?;
		let (a, b) = opening.rim_edge(edge);
		let edge_dir = (b - a).normalize_or_zero();
		if edge_dir.length_squared() < EPS * EPS {
			return None;
		}
		let along = if edge_dir.dot(end.travel) >= 0.0 { edge_dir } else { -edge_dir };
		let end_pt = if along.dot(b - a) >= 0.0 { b } else { a };
		let length = (end_pt - a0_xz).dot(along).min(RUN_IN_M);
		if length < MIN_SLAB_M {
			return None;
		}
		let lead = end.leading();
		let a1_xz = a0_xz + along * length;
		let b0_xz = a0_xz + lead;
		let b1_xz = b0_xz + along * length;
		let y = opening.walk_on_mid().y;
		Some(QuadPanel::slab(
			style,
			at_y(a0_xz, y),
			at_y(a1_xz, y),
			at_y(b0_xz, y),
			at_y(b1_xz, y),
			thickness,
		))
	}

}

impl StairwellFlight {
	/// Upper-landing slab on `opening`, or [`None`] if the rim is too short.
	///
	/// The last run is responsible for arriving at this rim. A sheared slab
	/// from a distant last tread is not authored here.
	pub fn landing_slab(
		&self,
		opening: StairwellOpening,
		style: PanelStyle,
		thickness: f32,
	) -> Option<QuadPanel> {
		self.tread_end().landing_slab(opening, style, thickness)
	}
}
