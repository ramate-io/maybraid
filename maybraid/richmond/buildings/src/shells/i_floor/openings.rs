//! Per-edge wall openings for [`IFloor`].

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::openings::{
	MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings,
};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{edge_score_for_bounds, standing_face_opening, WallEdge, EPS};

use super::{IFloor, IFloorParams};

impl IFloor {
	/// Thin passage AABB centered on a wall edge (by index into [`IFloor::edges`]).
	pub fn edge_passage_opening(edge: WallEdge, width: f32, height: f32) -> Opening {
		edge_opening(edge, width, height, OpeningLabel::Passage, 0.0)
	}

	/// Thin aperture AABB on a wall edge with sill height.
	pub fn edge_aperture_opening(edge: WallEdge, width: f32, height: f32, sill: f32) -> Opening {
		edge_opening(edge, width, height, OpeningLabel::Aperture, sill.max(0.0))
	}
}

fn edge_opening(
	edge: WallEdge,
	width: f32,
	height: f32,
	label: OpeningLabel,
	sill: f32,
) -> Opening {
	let width = width.max(1e-3).min(edge.length() * 0.95);
	let height = height.max(1e-3).min(edge.height * 0.95);
	let tang = edge.tangent();
	let outward = Vec3::new(edge.outward.x, 0.0, edge.outward.y);
	let mid = edge.mid() + Vec3::Y * sill;
	let half_w = width * 0.5;
	let depth = 0.4;
	let min = mid - tang * half_w - outward * depth * 0.25;
	let max = mid + tang * half_w + outward * depth + Vec3::Y * height;
	Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
}

impl IFloorParams {
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

impl MapsOpenings for IFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}
