//! Shared layout helpers for commercial stall interiors.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::bedroom::shell::face_rectangle;
use crate::constraints::FaceKind;
use crate::fit::{aabb_near_plane, aabb_xz_extent, Confines};
use crate::openings::{Opening, OpeningLabel};
use crate::paneling::Rectangle;
use crate::paneling::DEFAULT_PANEL_THICKNESS;

/// Plan cardinal used for façade / counter placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StallSide {
	South,
	North,
	East,
	West,
}

impl StallSide {
	#[allow(dead_code)]
	pub fn inward(self) -> Vec3 {
		match self {
			Self::South => Vec3::Z,
			Self::North => -Vec3::Z,
			Self::East => -Vec3::X,
			Self::West => Vec3::X,
		}
	}
}

/// Longest connectable façade opening, or the longest plan edge if none.
pub fn primary_facade(confines: &Confines) -> (StallSide, f32) {
	let mut best: Option<(StallSide, f32)> = None;
	for (_id, opening) in confines.openings.iter() {
		if !matches!(
			opening.label,
			OpeningLabel::Passage | OpeningLabel::Aperture
		) {
			continue;
		}
		let Some(side) = side_for_opening(&confines.bounds, opening) else {
			continue;
		};
		let len = opening_along_len(opening, side);
		if best.map(|(_, l)| len > l).unwrap_or(true) {
			best = Some((side, len));
		}
	}
	if let Some(b) = best {
		return b;
	}
	let e = aabb_xz_extent(&confines.bounds);
	if e.x >= e.y {
		(StallSide::South, e.x)
	} else {
		(StallSide::East, e.y)
	}
}

pub fn side_for_opening(bounds: &Aabb3d, opening: &Opening) -> Option<StallSide> {
	let bmin = Vec3::from(bounds.min);
	let bmax = Vec3::from(bounds.max);
	let omin = Vec3::from(opening.bounds.min);
	let omax = Vec3::from(opening.bounds.max);
	let oc = (omin + omax) * 0.5;
	let tol = 0.45_f32;
	let x_span = (omax.x - omin.x).abs();
	let z_span = (omax.z - omin.z).abs();
	// Prefer the face whose plane the opening hugs and whose *along* axis is the
	// longer plan span (avoids east/west doors near z=0 being labeled south).
	let mut best: Option<(StallSide, f32)> = None;
	let mut consider = |side: StallSide, on_plane: bool, dist: f32, along: f32, through: f32| {
		if !on_plane || along + 1e-3 < through {
			return;
		}
		if best.map(|(_, d)| dist < d - 1e-4).unwrap_or(true) {
			best = Some((side, dist));
		}
	};
	consider(
		StallSide::South,
		aabb_near_plane(omin.z, omax.z, bmin.z, tol),
		(oc.z - bmin.z).abs(),
		x_span,
		z_span,
	);
	consider(
		StallSide::North,
		aabb_near_plane(omin.z, omax.z, bmax.z, tol),
		(oc.z - bmax.z).abs(),
		x_span,
		z_span,
	);
	consider(
		StallSide::East,
		aabb_near_plane(omin.x, omax.x, bmax.x, tol),
		(oc.x - bmax.x).abs(),
		z_span,
		x_span,
	);
	consider(
		StallSide::West,
		aabb_near_plane(omin.x, omax.x, bmin.x, tol),
		(oc.x - bmin.x).abs(),
		z_span,
		x_span,
	);
	best.map(|(side, _)| side)
}

pub fn opening_along_len(opening: &Opening, side: StallSide) -> f32 {
	let omin = Vec3::from(opening.bounds.min);
	let omax = Vec3::from(opening.bounds.max);
	match side {
		StallSide::South | StallSide::North => (omax.x - omin.x).abs(),
		StallSide::East | StallSide::West => (omax.z - omin.z).abs(),
	}
}

/// Inflate `aabb` on XZ by `pad` (Y unchanged).
pub fn inflate_xz(aabb: &Aabb3d, pad: f32) -> Aabb3d {
	let min = Vec3::from(aabb.min);
	let max = Vec3::from(aabb.max);
	let pad = pad.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(min.x - pad, min.y, min.z - pad),
		Vec3::new(max.x + pad, max.y, max.z + pad),
	)
}

/// Largest AABB inside `bounds` that stays ≥`clearance` (XZ) from every obstacle.
///
/// Enumerates candidate rectangles from the obstacle/bounds edge grid so gaps
/// between counters are claimed when they clear the buffers.
pub fn largest_remainder_away_from(
	bounds: &Aabb3d,
	obstacles: &[Aabb3d],
	clearance: f32,
) -> Option<Aabb3d> {
	let cuts: Vec<Aabb3d> = obstacles.iter().map(|o| inflate_xz(o, clearance)).collect();
	largest_aabb_avoiding_xz(bounds, &cuts)
}

fn aabb_xz_area(aabb: &Aabb3d) -> f32 {
	let e = aabb_xz_extent(aabb);
	e.x * e.y
}

fn aabb_xz_overlap(a: &Aabb3d, b: &Aabb3d) -> bool {
	let amin = Vec3::from(a.min);
	let amax = Vec3::from(a.max);
	let bmin = Vec3::from(b.min);
	let bmax = Vec3::from(b.max);
	amin.x < bmax.x - 1e-4
		&& bmin.x < amax.x - 1e-4
		&& amin.z < bmax.z - 1e-4
		&& bmin.z < amax.z - 1e-4
}

fn largest_aabb_avoiding_xz(bounds: &Aabb3d, cuts: &[Aabb3d]) -> Option<Aabb3d> {
	let bmin = Vec3::from(bounds.min);
	let bmax = Vec3::from(bounds.max);
	let mut xs = vec![bmin.x, bmax.x];
	let mut zs = vec![bmin.z, bmax.z];
	for cut in cuts {
		let cmin = Vec3::from(cut.min);
		let cmax = Vec3::from(cut.max);
		xs.push(cmin.x.clamp(bmin.x, bmax.x));
		xs.push(cmax.x.clamp(bmin.x, bmax.x));
		zs.push(cmin.z.clamp(bmin.z, bmax.z));
		zs.push(cmax.z.clamp(bmin.z, bmax.z));
	}
	xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	xs.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
	zs.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

	let mut best: Option<(f32, Aabb3d)> = None;
	for i in 0..xs.len() {
		for j in (i + 1)..xs.len() {
			for k in 0..zs.len() {
				for l in (k + 1)..zs.len() {
					let cand = Aabb3d::from_min_max(
						Vec3::new(xs[i], bmin.y, zs[k]),
						Vec3::new(xs[j], bmax.y, zs[l]),
					);
					if xs[j] - xs[i] < 1e-3 || zs[l] - zs[k] < 1e-3 {
						continue;
					}
					if cuts.iter().any(|c| aabb_xz_overlap(&cand, c)) {
						continue;
					}
					let area = aabb_xz_area(&cand);
					if best.map(|(a, _)| area > a + 1e-6).unwrap_or(true) {
						best = Some((area, cand));
					}
				}
			}
		}
	}
	best.map(|(_, aabb)| aabb)
}

/// Counter band on `side` of depth `depth`, covering `along_len` centered in the
/// opening’s along-span (clamped to `bounds`).
pub fn counter_on_opening(
	bounds: &Aabb3d,
	opening: &Opening,
	side: StallSide,
	depth: f32,
	along_len: f32,
) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let omin = Vec3::from(opening.bounds.min);
	let omax = Vec3::from(opening.bounds.max);
	let depth = depth
		.min(match side {
			StallSide::South | StallSide::North => ((max.z - min.z) * 0.45).max(0.35),
			StallSide::East | StallSide::West => ((max.x - min.x) * 0.45).max(0.35),
		})
		.max(0.35);
	let along_len = along_len.max(0.2);
	match side {
		StallSide::South | StallSide::North => {
			let span0 = omin.x.clamp(min.x, max.x);
			let span1 = omax.x.clamp(min.x, max.x).max(span0 + 0.2);
			let mid = (span0 + span1) * 0.5;
			let half = (along_len * 0.5).min((span1 - span0) * 0.5);
			let x0 = (mid - half).clamp(span0, span1);
			let x1 = (mid + half).clamp(span0, span1).max(x0 + 0.2);
			if matches!(side, StallSide::South) {
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, min.z),
					Vec3::new(x1, max.y, min.z + depth),
				)
			} else {
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, max.z - depth),
					Vec3::new(x1, max.y, max.z),
				)
			}
		}
		StallSide::East | StallSide::West => {
			let span0 = omin.z.clamp(min.z, max.z);
			let span1 = omax.z.clamp(min.z, max.z).max(span0 + 0.2);
			let mid = (span0 + span1) * 0.5;
			let half = (along_len * 0.5).min((span1 - span0) * 0.5);
			let z0 = (mid - half).clamp(span0, span1);
			let z1 = (mid + half).clamp(span0, span1).max(z0 + 0.2);
			if matches!(side, StallSide::East) {
				Aabb3d::from_min_max(
					Vec3::new(max.x - depth, min.y, z0),
					Vec3::new(max.x, max.y, z1),
				)
			} else {
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, z0),
					Vec3::new(min.x + depth, max.y, z1),
				)
			}
		}
	}
}

/// Band along `side` of depth `depth`, covering `cover` fraction of that edge (centered).
pub fn facade_band(bounds: &Aabb3d, side: StallSide, depth: f32, cover: f32) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let cover = cover.clamp(0.35, 0.95);
	let depth = depth
		.min(match side {
			StallSide::South | StallSide::North => (max.z - min.z) * 0.45,
			StallSide::East | StallSide::West => (max.x - min.x) * 0.45,
		})
		.max(0.35);
	match side {
		StallSide::South => {
			let span = (max.x - min.x) * cover;
			let x0 = (min.x + max.x) * 0.5 - span * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(x0, min.y, min.z),
				Vec3::new(x0 + span, max.y, min.z + depth),
			)
		}
		StallSide::North => {
			let span = (max.x - min.x) * cover;
			let x0 = (min.x + max.x) * 0.5 - span * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(x0, min.y, max.z - depth),
				Vec3::new(x0 + span, max.y, max.z),
			)
		}
		StallSide::East => {
			let span = (max.z - min.z) * cover;
			let z0 = (min.z + max.z) * 0.5 - span * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(max.x - depth, min.y, z0),
				Vec3::new(max.x, max.y, z0 + span),
			)
		}
		StallSide::West => {
			let span = (max.z - min.z) * cover;
			let z0 = (min.z + max.z) * 0.5 - span * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(min.x, min.y, z0),
				Vec3::new(min.x + depth, max.y, z0 + span),
			)
		}
	}
}

/// Band inset from `side` by `offset`, depth `depth`, full edge cover.
pub fn inset_band(bounds: &Aabb3d, side: StallSide, offset: f32, depth: f32) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let depth = depth.max(0.3);
	match side {
		StallSide::South => {
			let z0 = (min.z + offset).min(max.z - depth);
			Aabb3d::from_min_max(
				Vec3::new(min.x, min.y, z0),
				Vec3::new(max.x, max.y, z0 + depth),
			)
		}
		StallSide::North => {
			let z1 = (max.z - offset).max(min.z + depth);
			Aabb3d::from_min_max(
				Vec3::new(min.x, min.y, z1 - depth),
				Vec3::new(max.x, max.y, z1),
			)
		}
		StallSide::East => {
			let x1 = (max.x - offset).max(min.x + depth);
			Aabb3d::from_min_max(
				Vec3::new(x1 - depth, min.y, min.z),
				Vec3::new(x1, max.y, max.z),
			)
		}
		StallSide::West => {
			let x0 = (min.x + offset).min(max.x - depth);
			Aabb3d::from_min_max(
				Vec3::new(x0, min.y, min.z),
				Vec3::new(x0 + depth, max.y, max.z),
			)
		}
	}
}

/// Back third of the footprint opposite `side` (office / kitchen residual).
pub fn back_third(bounds: &Aabb3d, side: StallSide) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	match side {
		StallSide::South => {
			let z0 = min.z + (max.z - min.z) * (2.0 / 3.0);
			Aabb3d::from_min_max(Vec3::new(min.x, min.y, z0), max)
		}
		StallSide::North => {
			let z1 = min.z + (max.z - min.z) / 3.0;
			Aabb3d::from_min_max(min, Vec3::new(max.x, max.y, z1))
		}
		StallSide::East => {
			let x1 = min.x + (max.x - min.x) / 3.0;
			Aabb3d::from_min_max(min, Vec3::new(x1, max.y, max.z))
		}
		StallSide::West => {
			let x0 = min.x + (max.x - min.x) * (2.0 / 3.0);
			Aabb3d::from_min_max(Vec3::new(x0, min.y, min.z), max)
		}
	}
}

/// Thin divider wall between office (back third) and sales floor.
pub fn office_divider_wall(bounds: &Aabb3d, office: &Aabb3d, side: StallSide) -> Option<Rectangle> {
	let face = match side {
		StallSide::South => FaceKind::Back,
		StallSide::North => FaceKind::Front,
		StallSide::East => FaceKind::Left,
		StallSide::West => FaceKind::Right,
	};
	let omin = Vec3::from(office.min);
	let omax = Vec3::from(office.max);
	let bmin = Vec3::from(bounds.min);
	let bmax = Vec3::from(bounds.max);
	let divider = match side {
		StallSide::South => Aabb3d::from_min_max(
			Vec3::new(bmin.x, bmin.y, omin.z - 0.05),
			Vec3::new(bmax.x, bmax.y, omin.z + 0.05),
		),
		StallSide::North => Aabb3d::from_min_max(
			Vec3::new(bmin.x, bmin.y, omax.z - 0.05),
			Vec3::new(bmax.x, bmax.y, omax.z + 0.05),
		),
		StallSide::East => Aabb3d::from_min_max(
			Vec3::new(omax.x - 0.05, bmin.y, bmin.z),
			Vec3::new(omax.x + 0.05, bmax.y, bmax.z),
		),
		StallSide::West => Aabb3d::from_min_max(
			Vec3::new(omin.x - 0.05, bmin.y, bmin.z),
			Vec3::new(omin.x + 0.05, bmax.y, bmax.z),
		),
	};
	face_rectangle(&divider, face, DEFAULT_PANEL_THICKNESS)
}

/// Sales floor = bounds minus `office` (back third opposite the façade).
pub fn sales_minus_office(bounds: &Aabb3d, office: &Aabb3d, side: StallSide) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let omin = Vec3::from(office.min);
	let omax = Vec3::from(office.max);
	match side {
		StallSide::South => Aabb3d::from_min_max(
			min,
			Vec3::new(max.x, max.y, omin.z.max(min.z + 0.4)),
		),
		StallSide::North => Aabb3d::from_min_max(
			Vec3::new(min.x, min.y, omax.z.min(max.z - 0.4)),
			max,
		),
		// Office is the west third; sales is the east remainder.
		StallSide::East => Aabb3d::from_min_max(
			Vec3::new(omax.x.min(max.x - 0.4), min.y, min.z),
			max,
		),
		// Office is the east third; sales is the west remainder.
		StallSide::West => Aabb3d::from_min_max(
			min,
			Vec3::new(omin.x.max(min.x + 0.4), max.y, max.z),
		),
	}
}
