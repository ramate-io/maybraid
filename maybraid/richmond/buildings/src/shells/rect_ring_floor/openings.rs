//! Per-side wall openings for [`RectRingFloor`].
//!
//! Wide connectable openings author broad omissions along a ring side (no
//! separate omit-interval API). Assignment is **one opening → one side**: the
//! nearest outer/inner edge wins. An AABB that overlaps half the ring still
//! only clips that single winner; clear multiple sides with multiple openings.

use bevy_math::{Vec2, Vec3};

use crate::openings::{MappedOpenings, MapsOpenings, Opening, OpeningId, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
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

impl RectRingFloorParams {
	/// Map each connectable opening onto at most one edge among outer+inner sides.
	///
	/// Overlap with several walls does not multi-assign: only the nearest edge
	/// (tie-break: largest face score) receives the inset.
	pub(super) fn resolve_walls(
		&self,
		edges: &[WallEdge],
	) -> (Vec<ClippedRectangularStrip>, Openings, MappedOpenings) {
		let thickness = self.joint_thickness.max(1e-4);
		let n = edges.len();
		let mut best: Vec<Option<(f32, OpeningId, Opening)>> = vec![None; n];

		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let mut winner: Option<(usize, f32, f32)> = None; // idx, dist, score
			for (i, edge) in edges.iter().enumerate() {
				if edge.length() < EPS {
					continue;
				}
				let (dist, score) = edge_score_for_bounds(&opening.bounds, *edge);
				let replace = match winner {
					None => true,
					Some((_, prev_d, prev_s)) => {
						dist < prev_d - 1e-4 || (dist <= prev_d + 1e-4 && score > prev_s)
					}
				};
				if replace {
					winner = Some((i, dist, score));
				}
			}
			let Some((idx, _, score)) = winner else {
				continue;
			};
			let replace = match &best[idx] {
				None => true,
				Some((prev, ..)) => score > *prev,
			};
			if replace {
				best[idx] = Some((score, id.clone(), opening.clone()));
			}
		}

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut walls = Vec::with_capacity(n);
		for (i, edge) in edges.iter().enumerate() {
			let inset = if let Some((_, id, opening)) = best[i].take() {
				if let Some(face) = standing_face_opening(*edge, &opening.bounds, thickness) {
					mapped.insert(id.clone(), face.mapped);
					openings.insert(id, opening);
					Some(face.inset)
				} else {
					None
				}
			} else {
				None
			};
			if edge.length() < EPS {
				continue;
			}
			walls.push(ClippedRectangularStrip::from_nodes(
				self.style,
				[
					RectangularStripNode::new(edge.start, edge.height, thickness, 0.0),
					RectangularStripNode::new(edge.end, edge.height, thickness, 0.0),
				],
				[inset],
			));
		}

		(walls, openings, mapped)
	}
}

impl MapsOpenings for RectRingFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}
