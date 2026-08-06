//! Frame floor / ceiling bands with positioned `cuts_slab` holes.

use richmond_building_components::panels::PanelStyle;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::PanelPoint;
use crate::shells::ortho::{
	horizontal_slab_cut_xz, plan_rect_aabb2, plan_rect_from_aabb2, subtract_aabb2d, PlanRect,
};

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
					out.extend(resolve_pieces(
						self.style,
						plan,
						&self.openings,
						self.joint_thickness,
					));
				}
				out
			}
		}
	}
}

/// Cut all `cuts_slab` openings from a frame band, emitting one solid per residual.
///
/// Multiple holes on the same N/S band (corner shafts) each leave a gap; a single
/// framed inset cannot express that, so residuals replace the old largest-hole merge.
fn resolve_pieces(
	style: PanelStyle,
	plan: PlanRect,
	openings: &Openings,
	thickness: f32,
) -> Vec<RingSlabPiece> {
	let host = plan_rect_aabb2(plan);
	let cuts: Vec<_> = openings
		.iter()
		.filter_map(|(_id, o)| {
			if o.label.cuts_slab() {
				horizontal_slab_cut_xz(plan, &o.bounds)
			} else {
				None
			}
		})
		.collect();
	if cuts.is_empty() {
		return vec![RingSlabPiece(solid(style, plan, thickness))];
	}
	subtract_aabb2d(host, &cuts)
		.into_iter()
		.map(|region| RingSlabPiece(solid(style, plan_rect_from_aabb2(plan.y, region), thickness)))
		.collect()
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
