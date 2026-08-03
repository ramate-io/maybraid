//! Frame floor / ceiling bands with positioned `cuts_slab` holes.

use richmond_building_components::panels::PanelStyle;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::PanelPoint;
use crate::paneling::RectInset;
use crate::shells::ortho::{merge_slab_insets, PlanRect};

use super::geometry::PlanAabb;
use super::{RectRingFloorParams, RectRingFloorSlab, RingSlabPiece};

impl RectRingFloorParams {
	pub(super) fn resolve_slab_pieces(
		&self,
		slab: RectRingFloorSlab,
		rects: &[PlanAabb],
		y: f32,
	) -> Vec<RingSlabPiece> {
		match slab {
			RectRingFloorSlab::None => Vec::new(),
			RectRingFloorSlab::Solid => {
				let mut out = Vec::new();
				for r in rects {
					let plan = r.to_plan_rect(y);
					if let Some(piece) =
						resolve_piece(self.style, plan, &self.openings, self.joint_thickness)
					{
						out.push(piece);
					}
				}
				out
			}
		}
	}
}

fn resolve_piece(
	style: PanelStyle,
	plan: PlanRect,
	openings: &Openings,
	thickness: f32,
) -> Option<RingSlabPiece> {
	let cutting = openings.iter().filter_map(|(_id, o)| {
		if o.label.cuts_slab() {
			Some(o.bounds)
		} else {
			None
		}
	});
	match merge_slab_insets(plan, cutting) {
		None => Some(RingSlabPiece::Solid(solid(style, plan, thickness))),
		Some(None) => None,
		Some(Some(inset)) => Some(RingSlabPiece::Clipped(clipped(style, plan, thickness, inset))),
	}
}

fn corners(plan: PlanRect, thickness: f32) -> [PanelPoint; 4] {
	let t = thickness.max(1e-4);
	[
		PanelPoint::new(plan.sw(), t),
		PanelPoint::new(plan.se(), t),
		PanelPoint::new(plan.nw(), t),
		PanelPoint::new(plan.ne(), t),
	]
}

fn solid(style: PanelStyle, plan: PlanRect, thickness: f32) -> FittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	FittedRectangle::new(style, sw, se, nw, ne)
}

fn clipped(
	style: PanelStyle,
	plan: PlanRect,
	thickness: f32,
	inset: RectInset,
) -> ClippedFittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	ClippedFittedRectangle::new(style, sw, se, nw, ne, inset)
}
