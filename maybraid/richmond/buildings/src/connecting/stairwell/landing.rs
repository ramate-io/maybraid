//! Upper landing: parallelogram flush with a [`TreadEnd`], along the opening rim.

use richmond_building_components::panels::PanelStyle;

use crate::connecting::geom::EPS;
use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::SpiralFlight;

use super::opening::{at_y, StairwellOpening};
use super::tread::TreadEnd;
use super::{MIN_SLAB_M, RUN_IN_M};

impl TreadEnd {
	/// Thin slab starting on this leading edge, extruded along the nearby rim.
	pub fn landing_slab(
		self,
		opening: StairwellOpening,
		style: PanelStyle,
		thickness: f32,
	) -> Option<QuadPanel> {
		let (edge, a0_xz) = opening.nearest_rim(self.leading_outer)?;
		let (a, b) = opening.rim_edge(edge);
		let edge_dir = (b - a).normalize_or_zero();
		if edge_dir.length_squared() < EPS * EPS {
			return None;
		}
		let along = if edge_dir.dot(self.travel) >= 0.0 { edge_dir } else { -edge_dir };
		let end_pt = if along.dot(b - a) >= 0.0 { b } else { a };
		let length = (end_pt - a0_xz).dot(along).min(RUN_IN_M);
		if length < MIN_SLAB_M {
			return None;
		}
		let lead = self.leading();
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

impl SpiralFlight {
	/// Upper-landing slab for this spiral on `opening`, or [`None`] if the rim is too short.
	pub fn landing_slab(
		&self,
		opening: StairwellOpening,
		style: PanelStyle,
		thickness: f32,
	) -> Option<QuadPanel> {
		TreadEnd::from_spiral(self).landing_slab(opening, style, thickness)
	}
}
