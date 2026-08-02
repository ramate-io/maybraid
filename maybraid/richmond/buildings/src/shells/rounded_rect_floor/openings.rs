//! Wall opening resolution for rounded-rect straights (and optional corner clips).

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId, Openings,
};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{
	edge_score_for_bounds, standing_face_opening, OrthoSide, WallEdge, EPS,
};

use super::geometry::{RoundedRectCorner, RoundedRectGeom};
use super::{RoundedRectFloor, RoundedRectFloorParams};

/// Cardinal straight of a [`RoundedRectFloor`].
pub type RoundedRectFloorSide = OrthoSide;

impl RoundedRectFloor {
	pub fn side_passage_opening(
		side: RoundedRectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_passage_opening(
			side, center_xz, footprint, width, height,
		)
	}

	pub fn side_aperture_opening(
		side: RoundedRectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
		sill: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_aperture_opening(
			side, center_xz, footprint, width, height, sill,
		)
	}
}

impl RoundedRectFloorParams {
	pub(super) fn resolve_walls(
		&self,
		geom: &RoundedRectGeom,
	) -> (
		[ClippedRectangularStrip; 4],
		[ClippedRuledStrip; 4],
		Openings,
		MappedOpenings,
	) {
		let thickness = self.joint_thickness.max(1e-4);
		let mut best_straight: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];
		let mut best_corner: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];

		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let mut best_kind = None::<(bool, usize, f32, f32)>; // (is_corner, idx, dist, score)
			for (i, edge) in geom.straights.iter().enumerate() {
				if edge.length() < EPS {
					continue;
				}
				let (dist, score) = edge_score_for_bounds(&opening.bounds, *edge);
				let replace = match best_kind {
					None => true,
					Some((_, _, prev_d, prev_s)) => {
						dist < prev_d - 1e-4 || (dist <= prev_d + 1e-4 && score > prev_s)
					}
				};
				if replace {
					best_kind = Some((false, i, dist, score));
				}
			}
			for corner in RoundedRectCorner::all() {
				let i = corner.index();
				if geom.corner_bottom[i].len() < 2 {
					continue;
				}
				let mid = geom.corner_bottom[i][geom.corner_bottom[i].len() / 2];
				let mid_top = mid + Vec3::Y * geom.height;
				let edge = WallEdge::new(
					geom.corner_bottom[i][0],
					*geom.corner_bottom[i].last().unwrap(),
					geom.height,
					corner_outward(corner),
				);
				let mid3 = (mid + mid_top) * 0.5;
				let opening_mid = Vec3::from((opening.bounds.min + opening.bounds.max) * 0.5);
				let dist = opening_mid.distance_squared(mid3);
				let score = edge.length() * 0.1; // corners lose ties to straights of similar dist
				let replace = match best_kind {
					None => true,
					Some((_, _, prev_d, prev_s)) => {
						dist + 0.25 < prev_d || (dist <= prev_d + 1e-4 && score > prev_s)
					}
				};
				if replace {
					let _ = edge;
					best_kind = Some((true, i, dist, score));
				}
			}

			let Some((is_corner, idx, _, score)) = best_kind else {
				continue;
			};
			if is_corner {
				let replace = match &best_corner[idx] {
					None => true,
					Some((prev, ..)) => score > *prev,
				};
				if replace {
					best_corner[idx] = Some((score, id.clone(), opening.clone()));
				}
			} else {
				let replace = match &best_straight[idx] {
					None => true,
					Some((prev, ..)) => score > *prev,
				};
				if replace {
					best_straight[idx] = Some((score, id.clone(), opening.clone()));
				}
			}
		}

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut straight_insets = [None, None, None, None];
		for side in OrthoSide::all() {
			let idx = side.face_index();
			let Some((_, id, opening)) = best_straight[idx].take() else {
				continue;
			};
			let edge = geom.straights[idx];
			if edge.length() < EPS {
				continue;
			}
			let Some(face) = standing_face_opening(edge, &opening.bounds, thickness) else {
				continue;
			};
			straight_insets[idx] = Some(face.inset);
			mapped.insert(id.clone(), face.mapped);
			openings.insert(id, opening);
		}

		let mut corner_clips: [Option<Vec<Vec3>>; 4] = [None, None, None, None];
		for corner in RoundedRectCorner::all() {
			let idx = corner.index();
			let Some((_, id, opening)) = best_corner[idx].take() else {
				continue;
			};
			let bot = &geom.corner_bottom[idx];
			let top = &geom.corner_top[idx];
			if bot.len() < 2 {
				continue;
			}
			if let Some((clip, mapped_o)) =
				corner_clip_from_bounds(bot, top, &opening.bounds, corner_outward(corner))
			{
				corner_clips[idx] = Some(clip);
				mapped.insert(id.clone(), mapped_o);
				openings.insert(id, opening);
			}
		}

		let straights = OrthoSide::all().map(|side| {
			let idx = side.face_index();
			let edge = geom.straights[idx];
			if edge.length() < EPS {
				return ClippedRectangularStrip::new(self.style);
			}
			ClippedRectangularStrip::from_nodes(
				self.style,
				[
					RectangularStripNode::new(edge.start, edge.height, thickness, 0.0),
					RectangularStripNode::new(edge.end, edge.height, thickness, 0.0),
				],
				[straight_insets[idx]],
			)
		});

		let corners = RoundedRectCorner::all().map(|corner| {
			let idx = corner.index();
			let bot = &geom.corner_bottom[idx];
			let top = &geom.corner_top[idx];
			if bot.len() < 2 {
				return ClippedRuledStrip::new(self.style);
			}
			let bay_count = bot.len() - 1;
			let mut clips = vec![None; bay_count];
			if let Some(clip) = corner_clips[idx].clone() {
				// Apply on the middle bay for a single positioned void.
				let mid = bay_count / 2;
				clips[mid] = Some(clip);
			}
			ClippedRuledStrip::from_lines(self.style, bot.clone(), top.clone(), clips)
		});

		(straights, corners, openings, mapped)
	}
}

fn corner_outward(corner: RoundedRectCorner) -> Vec2 {
	match corner {
		RoundedRectCorner::SouthEast => Vec2::new(1.0, -1.0).normalize(),
		RoundedRectCorner::NorthEast => Vec2::new(1.0, 1.0).normalize(),
		RoundedRectCorner::NorthWest => Vec2::new(-1.0, 1.0).normalize(),
		RoundedRectCorner::SouthWest => Vec2::new(-1.0, -1.0).normalize(),
	}
}

fn corner_clip_from_bounds(
	bot: &[Vec3],
	top: &[Vec3],
	bounds: &Aabb3d,
	outward: Vec2,
) -> Option<(Vec<Vec3>, MappedOpening)> {
	let n = bot.len();
	if n < 2 || top.len() != n {
		return None;
	}
	let y0 = bot[0].y;
	let y1 = top[0].y;
	let imin = Vec3::from(bounds.min);
	let imax = Vec3::from(bounds.max);
	let hy0 = imin.y.clamp(y0, y1);
	let hy1 = imax.y.clamp(y0, y1);
	if hy1 - hy0 < EPS {
		return None;
	}

	// Pick the chord stations whose midpoints are closest to the opening center in XZ.
	let mid_xz = Vec2::new(
		0.5 * (imin.x + imax.x),
		0.5 * (imin.z + imax.z),
	);
	let mut best_i = 0usize;
	let mut best_d = f32::MAX;
	for i in 0..n - 1 {
		let m = (bot[i] + bot[i + 1]) * 0.5;
		let d = (Vec2::new(m.x, m.z) - mid_xz).length_squared();
		if d < best_d {
			best_d = d;
			best_i = i;
		}
	}
	let a0 = bot[best_i];
	let b0 = bot[best_i + 1];
	let a1 = top[best_i];
	let b1 = top[best_i + 1];
	let v0 = ((hy0 - y0) / (y1 - y0).max(EPS)).clamp(0.0, 1.0);
	let v1 = ((hy1 - y0) / (y1 - y0).max(EPS)).clamp(0.0, 1.0);
	let bl = a0.lerp(a1, v0);
	let br = b0.lerp(b1, v0);
	let tl = a0.lerp(a1, v1);
	let tr = b0.lerp(b1, v1);
	let clip = vec![bl, br, tr, tl];
	let mapped = MappedOpening::new(MappedOpeningQuad::new(br, bl, tr, tl), outward);
	Some((clip, mapped))
}

impl MapsOpenings for RoundedRectFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}
