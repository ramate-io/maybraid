//! Floor / ceiling: inset rectangular core + quarter-disk fans; positioned cuts.

use bevy_math::Vec3;
use richmond_building_components::panels::PanelStyle;
use std::f32::consts::FRAC_PI_2;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::{PanelComplex, PanelPoint};
use crate::paneling::RectInset;
use crate::shells::ortho::{merge_slab_insets, PlanRect, EPS};

use super::geometry::{core_plan, RoundedRectCorner};
use super::{RoundedRectFloorParams, RoundedRectFloorSlab, RoundedSlabPiece};

impl RoundedRectFloorParams {
	pub(super) fn resolve_slab_parts(
		&self,
		slab: RoundedRectFloorSlab,
		plan: PlanRect,
		radius: f32,
	) -> (Option<RoundedSlabPiece>, Vec<PanelComplex>) {
		match slab {
			RoundedRectFloorSlab::None => (None, Vec::new()),
			RoundedRectFloorSlab::Solid => {
				let core = core_plan(plan, radius);
				let cutting: Vec<_> = self
					.openings
					.iter()
					.filter_map(|(_id, o)| o.label.cuts_slab().then_some(o.bounds))
					.collect();
				let core_piece = match merge_slab_insets(core, cutting.iter().copied()) {
					None => Some(RoundedSlabPiece::Solid(solid_rect(
						self.style,
						core,
						self.joint_thickness,
					))),
					Some(None) => None,
					Some(Some(inset)) => Some(RoundedSlabPiece::Clipped(clipped_rect(
						self.style,
						core,
						self.joint_thickness,
						inset,
					))),
				};

				let mut quarters = Vec::new();
				if radius > EPS {
					for corner in RoundedRectCorner::all() {
						if let Some(q) = quarter_disk(
							self.style,
							plan,
							radius,
							self.corner_segments.max(1),
							corner,
							self.joint_thickness,
							&self.openings,
						) {
							quarters.push(q);
						}
					}
				}
				(core_piece, quarters)
			}
		}
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

fn solid_rect(style: PanelStyle, plan: PlanRect, thickness: f32) -> FittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	FittedRectangle::new(style, sw, se, nw, ne)
}

fn clipped_rect(
	style: PanelStyle,
	plan: PlanRect,
	thickness: f32,
	inset: RectInset,
) -> ClippedFittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	ClippedFittedRectangle::new(style, sw, se, nw, ne, inset)
}

/// Quarter disk at a corner. Omitted entirely when a cuts_slab opening covers its AABB.
fn quarter_disk(
	style: PanelStyle,
	plan: PlanRect,
	radius: f32,
	segments: u32,
	corner: RoundedRectCorner,
	thickness: f32,
	openings: &Openings,
) -> Option<PanelComplex> {
	let c = corner.center(plan, radius);
	let start = corner.start_angle();
	let n = segments.max(1);
	// Rough AABB of the quarter for coverage tests.
	let mut amin = c;
	let mut amax = c;
	for i in 0..=n {
		let t = i as f32 / n as f32;
		let ang = start + t * FRAC_PI_2;
		let p = Vec3::new(c.x + ang.cos() * radius, plan.y, c.z + ang.sin() * radius);
		amin = amin.min(p);
		amax = amax.max(p);
	}
	let qplan = PlanRect::new(
		Vec3::new(0.5 * (amin.x + amax.x), plan.y, 0.5 * (amin.z + amax.z)),
		(amax.x - amin.x).max(EPS),
		(amax.z - amin.z).max(EPS),
	);
	for (_id, o) in openings.iter() {
		if !o.label.cuts_slab() {
			continue;
		}
		if let Some(None) = merge_slab_insets(qplan, std::iter::once(o.bounds)) {
			return None;
		}
	}

	let t = thickness.max(1e-4);
	let mut complex = PanelComplex::new(style);
	let center_id = complex.insert_point_thick(c, t);
	let mut ring = Vec::with_capacity(n as usize + 1);
	for i in 0..=n {
		let frac = i as f32 / n as f32;
		let ang = start + frac * FRAC_PI_2;
		let p = Vec3::new(c.x + ang.cos() * radius, plan.y, c.z + ang.sin() * radius);
		ring.push(complex.insert_point_thick(p, t));
	}
	for w in ring.windows(2) {
		complex.add_triangle(center_id, w[0], w[1]);
	}
	Some(complex)
}
