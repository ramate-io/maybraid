//! Per-side wall openings for [`RectRingFloor`].
//!
//! Wide connectable openings author broad omissions along a ring side (no
//! separate omit-interval API). Assignment is **nearest edge wins** per opening;
//! **multiple openings may land on the same side** — the wall strip is subdivided
//! into solid and opening bays along the run. An AABB that overlaps half the ring
//! still only assigns to that single nearest side; clear multiple sides with
//! multiple openings.

use bevy_math::{Vec2, Vec3};

use crate::openings::{
	MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings,
};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{edge_score_for_bounds, standing_face_opening, OrthoSide, WallEdge, EPS};

use super::{RectRingFloor, RectRingFloorParams};

/// Cardinal side of a [`RectRingFloor`] (alias of [`OrthoSide`]).
pub type RectRingFloorSide = OrthoSide;

impl RectRingFloor {
	/// Passage AABB centered on an outer cardinal side.
	///
	/// Pass a large `width` (near the side length) to author a broad omission
	/// along **that** side only. This helper does not fan out to adjacent sides.
	pub fn side_passage_opening(
		side: RectRingFloorSide,
		center_xz: Vec3,
		outer: Vec2,
		width: f32,
		height: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_passage_opening(
			side, center_xz, outer, width, height,
		)
	}

	/// Aperture AABB on an outer cardinal side with sill height.
	pub fn side_aperture_opening(
		side: RectRingFloorSide,
		center_xz: Vec3,
		outer: Vec2,
		width: f32,
		height: f32,
		sill: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_aperture_opening(
			side, center_xz, outer, width, height, sill,
		)
	}
}

struct EdgeOpening {
	id: OpeningId,
	opening: Opening,
	/// Along-wall start / end of the opening on this edge.
	s_lo: f32,
	s_hi: f32,
	/// Vertical margins (standing-strip left/right).
	sill: f32,
	header: f32,
	mapped: crate::openings::MappedOpening,
}

impl RectRingFloorParams {
	/// Map connectable openings onto outer+inner edges (multi-opening per edge).
	pub(super) fn resolve_walls(
		&self,
		edges: &[WallEdge],
	) -> (Vec<ClippedRectangularStrip>, Openings, MappedOpenings) {
		let thickness = self.joint_thickness.max(1e-4);
		let n = edges.len();
		let mut per_edge: Vec<Vec<EdgeOpening>> = (0..n).map(|_| Vec::new()).collect();

		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			// Only edges that actually project the AABB into a face opening — nearest
			// edge by midpoint alone can pick a side the void never intersects
			// (e.g. a SE corner door closer to East mid than South mid).
			let mut winner: Option<(usize, f32, f32, EdgeOpening)> = None;
			for (i, edge) in edges.iter().enumerate() {
				if edge.length() < EPS {
					continue;
				}
				let Some(face) = standing_face_opening(*edge, &opening.bounds, thickness) else {
					continue;
				};
				let len = edge.length();
				let s_lo = face.inset.bottom.clamp(0.0, len);
				let s_hi = (len - face.inset.top).clamp(0.0, len);
				if s_hi - s_lo < EPS {
					continue;
				}
				let (dist, score) = edge_score_for_bounds(&opening.bounds, *edge);
				let replace = match winner {
					None => true,
					Some((_, prev_d, prev_s, _)) => {
						dist < prev_d - 1e-4 || (dist <= prev_d + 1e-4 && score > prev_s)
					}
				};
				if replace {
					winner = Some((
						i,
						dist,
						score,
						EdgeOpening {
							id: id.clone(),
							opening: opening.clone(),
							s_lo,
							s_hi,
							sill: face.inset.left,
							header: face.inset.right,
							mapped: face.mapped,
						},
					));
				}
			}
			let Some((idx, _, _, edge_op)) = winner else {
				continue;
			};
			per_edge[idx].push(edge_op);
		}

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut walls = Vec::with_capacity(n);
		for (i, edge) in edges.iter().enumerate() {
			if edge.length() < EPS {
				continue;
			}
			let mut assigned = std::mem::take(&mut per_edge[i]);
			// Place higher-priority labels first (passages beat apertures), then
			// fill remaining spans so generated windows don't erase inbound doors.
			assigned.sort_by(|a, b| {
				connectable_priority(&b.opening.label)
					.cmp(&connectable_priority(&a.opening.label))
					.then_with(|| {
						a.s_lo
							.partial_cmp(&b.s_lo)
							.unwrap_or(std::cmp::Ordering::Equal)
					})
			});
			let mut kept = Vec::new();
			for op in assigned {
				if kept.iter().any(|k: &EdgeOpening| spans_overlap(k.s_lo, k.s_hi, op.s_lo, op.s_hi))
				{
					continue;
				}
				kept.push(op);
			}
			kept.sort_by(|a, b| {
				a.s_lo
					.partial_cmp(&b.s_lo)
					.unwrap_or(std::cmp::Ordering::Equal)
			});
			for op in &kept {
				mapped.insert(op.id.clone(), op.mapped.clone());
				openings.insert(op.id.clone(), op.opening.clone());
			}
			walls.push(wall_strip_for_edge(*edge, &kept, self.style, thickness));
		}

		(walls, openings, mapped)
	}
}

fn connectable_priority(label: &OpeningLabel) -> u8 {
	match label {
		OpeningLabel::Passage | OpeningLabel::Shaft => 2,
		OpeningLabel::Aperture => 1,
		_ => 0,
	}
}

fn spans_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
	a0 < b1 - EPS && b0 < a1 - EPS
}

fn wall_strip_for_edge(
	edge: WallEdge,
	openings: &[EdgeOpening],
	style: richmond_building_components::panels::PanelStyle,
	thickness: f32,
) -> ClippedRectangularStrip {
	let len = edge.length();
	let tang = edge.tangent();
	let h = edge.height;
	let t = thickness.max(1e-4);

	if openings.is_empty() {
		return ClippedRectangularStrip::from_nodes(
			style,
			[
				RectangularStripNode::new(edge.start, h, t, 0.0),
				RectangularStripNode::new(edge.end, h, t, 0.0),
			],
			[None],
		);
	}

	let mut nodes = Vec::new();
	let mut insets: Vec<Option<RectInset>> = Vec::new();
	nodes.push(RectangularStripNode::new(edge.start, h, t, 0.0));
	let mut cursor = 0.0_f32;

	for op in openings {
		if op.s_lo > cursor + EPS {
			nodes.push(RectangularStripNode::new(
				edge.start + tang * op.s_lo,
				h,
				t,
				0.0,
			));
			insets.push(None);
			cursor = op.s_lo;
		}
		let s_hi = op.s_hi.max(cursor + EPS);
		nodes.push(RectangularStripNode::new(edge.start + tang * s_hi, h, t, 0.0));
		// Bay spans the opening along-wall; vertical margins from the face projection.
		// Tiny along-wall jambs avoid [`RectInset::is_solid`] (all-zero ⇒ solid fill).
		let jamb = 0.02_f32.min((s_hi - cursor) * 0.1);
		insets.push(Some(RectInset::new(op.sill, op.header, jamb, jamb)));
		cursor = s_hi;
	}

	if cursor < len - EPS {
		nodes.push(RectangularStripNode::new(edge.end, h, t, 0.0));
		insets.push(None);
	} else if let Some(last) = nodes.last_mut() {
		last.position = edge.end;
	}

	ClippedRectangularStrip::from_nodes(style, nodes, insets)
}

impl MapsOpenings for RectRingFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}
