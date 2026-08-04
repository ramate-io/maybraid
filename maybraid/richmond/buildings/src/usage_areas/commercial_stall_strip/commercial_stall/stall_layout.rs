//! Shared layout helpers for commercial stall interiors.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, grow_aabb2_pair, inflate_aabb2,
	max_empty_aabb3_plan, max_empty_rect2, max_empty_rect2_by, plan_to_aabb3, touches_aabb2,
	PlanAxes,
};

use crate::bedroom::shell::face_rectangle;
use crate::constraints::FaceKind;
use crate::fit::{aabb_near_plane, aabb_xz_extent, Confines, FitError};
use crate::openings::{Opening, OpeningLabel};
use crate::paneling::Rectangle;
use crate::paneling::DEFAULT_PANEL_THICKNESS;

/// Passages must be at least this long (along-wall) to host a BitesCounter.
pub const BITES_LONG_PASSAGE_MIN: f32 = 2.0;
/// Counter along-length floor; the rest of the passage (≥1m) stays clear.
pub const BITES_COUNTER_ALONG_MIN: f32 = 1.0;
/// Clear passage length left beside each counter.
pub const BITES_PASSAGE_REMAIN_MIN: f32 = 1.0;
/// Kitchen stays at least this far (XZ) from every counter.
pub const BITES_KITCHEN_COUNTER_CLEARANCE: f32 = 1.0;
/// Kitchen / seating plan minimum (width and depth).
pub const BITES_REGION_MIN_PLAN: f32 = 1.0;

/// Counters packed on long passages, plus the passages they used.
#[derive(Debug, Clone)]
pub struct PackedBitesCounters {
	pub counters: Vec<Aabb3d>,
	pub passages: Vec<Aabb3d>,
}

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

/// Largest plan AABB inside `bounds` that stays ≥`clearance` (XZ) from every obstacle.
///
/// Thin wrapper over [`procedural_common::max_empty_aabb3_plan`].
pub fn largest_remainder_away_from(
	bounds: &Aabb3d,
	obstacles: &[Aabb3d],
	clearance: f32,
) -> Option<Aabb3d> {
	max_empty_aabb3_plan(bounds, obstacles, PlanAxes::XZ, clearance)
}

/// Place BitesCounters on every Passage ≥ [`BITES_LONG_PASSAGE_MIN`], each leaving
/// ≥ [`BITES_PASSAGE_REMAIN_MIN`] of that passage clear.
pub fn pack_bites_counters(
	confines: &Confines,
	counter_depth: f32,
) -> Result<PackedBitesCounters, FitError> {
	let mut counters = Vec::new();
	let mut passages = Vec::new();
	for (_id, opening) in confines.openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let Some(side) = side_for_opening(&confines.bounds, opening) else {
			continue;
		};
		let passage_len = opening_along_len(opening, side);
		if passage_len + 1e-3 < BITES_LONG_PASSAGE_MIN {
			continue;
		}
		let along = (passage_len - BITES_PASSAGE_REMAIN_MIN).max(BITES_COUNTER_ALONG_MIN);
		if along + 1e-3 < BITES_COUNTER_ALONG_MIN
			|| passage_len - along + 1e-3 < BITES_PASSAGE_REMAIN_MIN
		{
			continue;
		}
		counters.push(counter_on_opening(
			&confines.bounds,
			opening,
			side,
			counter_depth,
			along,
		));
		passages.push(opening.bounds);
	}
	if counters.is_empty() {
		return Err(FitError::TooSmall {
			reason: "bites counter passage",
		});
	}
	Ok(PackedBitesCounters {
		counters,
		passages,
	})
}

/// Largest ≥`min_plan` region avoiding `excludes` (no clearance) that **touches**
/// at least one of `passages` in plan (edge contact counts).
pub fn pack_passage_connected_region(
	bounds: &Aabb3d,
	excludes: &[Aabb3d],
	passages: &[Aabb3d],
	min_plan: f32,
) -> Option<Aabb3d> {
	if passages.is_empty() {
		return None;
	}
	let host = aabb3_to_plan(bounds, PlanAxes::XZ);
	let cuts: Vec<_> = excludes
		.iter()
		.map(|e| aabb3_to_plan(e, PlanAxes::XZ))
		.collect();
	let passage_plans: Vec<_> = passages
		.iter()
		.map(|p| aabb3_to_plan(p, PlanAxes::XZ))
		.collect();
	let min_plan = min_plan.max(0.0);
	let region2 = max_empty_rect2_by(host, &cuts, |cand| {
		let size = cand.max - cand.min;
		if size.x + 1e-3 < min_plan || size.y + 1e-3 < min_plan {
			return f32::NEG_INFINITY;
		}
		if !passage_plans.iter().any(|p| touches_aabb2(cand, *p)) {
			return f32::NEG_INFINITY;
		}
		aabb2_area(cand)
	})?;
	clamp_min_size2(region2, Vec2::splat(min_plan))?;
	Some(plan_to_aabb3(bounds, region2, PlanAxes::XZ))
}

/// Kitchen remainder: ≥1m from counters, may abut `extra_excludes` (e.g. seating).
pub fn pack_bites_kitchen(
	bounds: &Aabb3d,
	counters: &[Aabb3d],
	extra_excludes: &[Aabb3d],
	min_plan: f32,
) -> Option<Aabb3d> {
	if extra_excludes.is_empty() {
		return largest_remainder_away_from(bounds, counters, BITES_KITCHEN_COUNTER_CLEARANCE)
			.and_then(|k| {
				let e = aabb_xz_extent(&k);
				(e.x + 1e-3 >= min_plan && e.y + 1e-3 >= min_plan).then_some(k)
			});
	}
	let host = aabb3_to_plan(bounds, PlanAxes::XZ);
	let mut cuts: Vec<_> = counters
		.iter()
		.map(|c| inflate_aabb2(aabb3_to_plan(c, PlanAxes::XZ), BITES_KITCHEN_COUNTER_CLEARANCE))
		.collect();
	cuts.extend(
		extra_excludes
			.iter()
			.map(|e| aabb3_to_plan(e, PlanAxes::XZ)),
	);
	let kitchen2 = max_empty_rect2(host, &cuts)?;
	let kitchen2 = clamp_min_size2(kitchen2, Vec2::splat(min_plan))?;
	Some(plan_to_aabb3(bounds, kitchen2, PlanAxes::XZ))
}

/// Sit-down regions: kitchen seed (clearance from counters) → seating seed
/// (passage-touching in the remainder) → [`grow_aabb2_pair`] so leftover dead
/// space is absorbed under each side's hard constraints.
///
/// Seating-first max-empty often claims the whole free volume and starves the
/// kitchen; kitchen-first leaves the near-counter / passage band for seating.
pub fn pack_bites_sitdown_regions(
	bounds: &Aabb3d,
	counters: &[Aabb3d],
	passages: &[Aabb3d],
	min_plan: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let kitchen_seed = pack_bites_kitchen(bounds, counters, &[], min_plan)?;
	let mut seating_excludes = counters.to_vec();
	seating_excludes.push(kitchen_seed);
	let seating_seed =
		pack_passage_connected_region(bounds, &seating_excludes, passages, min_plan)?;

	let host = aabb3_to_plan(bounds, PlanAxes::XZ);
	let counter_plans: Vec<_> = counters
		.iter()
		.map(|c| aabb3_to_plan(c, PlanAxes::XZ))
		.collect();
	let kitchen_hard: Vec<_> = counter_plans
		.iter()
		.copied()
		.map(|c| inflate_aabb2(c, BITES_KITCHEN_COUNTER_CLEARANCE))
		.collect();
	let seating2 = aabb3_to_plan(&seating_seed, PlanAxes::XZ);
	let kitchen2 = aabb3_to_plan(&kitchen_seed, PlanAxes::XZ);
	// Grow seating first so the passage-band claims lateral scraps; kitchen
	// then expands into whatever remains outside the 1m counter halo.
	let (seating2, kitchen2) = grow_aabb2_pair(
		host,
		seating2,
		kitchen2,
		&counter_plans,
		&kitchen_hard,
		8,
	);
	let seating2 = clamp_min_size2(seating2, Vec2::splat(min_plan))?;
	let kitchen2 = clamp_min_size2(kitchen2, Vec2::splat(min_plan))?;
	Some((
		plan_to_aabb3(bounds, seating2, PlanAxes::XZ),
		plan_to_aabb3(bounds, kitchen2, PlanAxes::XZ),
	))
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
