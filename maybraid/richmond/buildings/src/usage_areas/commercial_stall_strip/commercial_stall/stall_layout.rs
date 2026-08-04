//! Shared layout helpers for commercial stall interiors.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, grow_aabb2, grow_aabb2_toward_area, inflate_aabb2,
	max_empty_aabb3_plan, max_empty_rect2, pack_optional_face_bands, passage_opening_face,
	plan_to_aabb3, seed_from_free_opening_face, shared_opening_border_len, OptionalFaceBand,
	PlanAxes, PlanOpeningFace,
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

/// Counters packed on long passages, plus every eligible passage (for seating faces).
#[derive(Debug, Clone)]
pub struct PackedBitesCounters {
	pub counters: Vec<Aabb3d>,
	/// All Passage openings that were long enough to host a counter (placed or not).
	#[allow(dead_code)]
	pub passages: Vec<Aabb3d>,
	pub faces: Vec<PlanOpeningFace>,
}

/// Per-passage counter stylization (parallel to [`eligible_bites_passages`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitesCounterChoice {
	pub place: bool,
	pub along: f32,
	pub depth: f32,
	pub along_t: f32,
}

/// A Passage long enough for a Bites counter, with its opening face into the stall.
#[derive(Debug, Clone)]
pub struct EligibleBitesPassage {
	pub bounds: Aabb3d,
	pub face: PlanOpeningFace,
	pub along_len: f32,
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

/// Passages ≥ [`BITES_LONG_PASSAGE_MIN`] with a resolvable opening face into the stall.
pub fn eligible_bites_passages(confines: &Confines) -> Vec<EligibleBitesPassage> {
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let mut out = Vec::new();
	for (_id, opening) in confines.openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let Some(side) = side_for_opening(&confines.bounds, opening) else {
			continue;
		};
		let along_len = opening_along_len(opening, side);
		if along_len + 1e-3 < BITES_LONG_PASSAGE_MIN {
			continue;
		}
		let passage_plan = aabb3_to_plan(&opening.bounds, PlanAxes::XZ);
		let Some(face) = passage_opening_face(host, passage_plan) else {
			continue;
		};
		if face.along_len() + 1e-3 < BITES_COUNTER_ALONG_MIN {
			continue;
		}
		out.push(EligibleBitesPassage {
			bounds: opening.bounds,
			face,
			along_len,
		});
	}
	out
}

/// Sample noisy counter choices for `eligible` passages (≥1 forced on).
pub fn sample_bites_counter_choices(
	eligible: &[EligibleBitesPassage],
	cfg: &procedural_common::NoiseConfig,
	origin: Vec3,
	salt: f32,
) -> Vec<BitesCounterChoice> {
	let mut choices = Vec::with_capacity(eligible.len());
	for (i, e) in eligible.iter().enumerate() {
		let k = salt + i as f32 * 17.0;
		let place_u = cfg.sample_range_f32_4d(0.0, 1.0, origin.x, origin.y, origin.z, k);
		// ~60% of long passages get a counter; depth/along still vary when placed.
		let place = place_u < 0.60;
		let max_along = (e.along_len - BITES_PASSAGE_REMAIN_MIN).max(BITES_COUNTER_ALONG_MIN);
		let along = cfg.sample_range_f32_4d(
			BITES_COUNTER_ALONG_MIN,
			max_along,
			origin.x,
			origin.y,
			origin.z,
			k + 1.0,
		);
		let depth =
			cfg.sample_range_f32_4d(0.65, 1.0, origin.x, origin.y, origin.z, k + 2.0);
		// Flush to one end so the clear remain stays one ≥1m face segment
		// (needed for sit-down seating contact). Interior along_t only when the
		// counter is short enough to leave ≥1m free on both sides.
		let remain = (e.along_len - along).max(0.0);
		let along_t = if remain + 1e-3 >= 2.0 * BITES_PASSAGE_REMAIN_MIN {
			cfg.sample_range_f32_4d(0.05, 0.95, origin.x, origin.y, origin.z, k + 3.0)
		} else if cfg.sample_range_f32_4d(0.0, 1.0, origin.x, origin.y, origin.z, k + 3.0) < 0.5
		{
			0.0
		} else {
			1.0
		};
		choices.push(BitesCounterChoice {
			place,
			along,
			depth,
			along_t,
		});
	}
	if !choices.is_empty() && !choices.iter().any(|c| c.place) {
		let best = eligible
			.iter()
			.enumerate()
			.max_by(|(_, a), (_, b)| {
				a.along_len
					.partial_cmp(&b.along_len)
					.unwrap_or(std::cmp::Ordering::Equal)
			})
			.map(|(i, _)| i)
			.unwrap_or(0);
		choices[best].place = true;
	}
	choices
}

/// Place counters from per-passage choices (skips `place: false`).
pub fn pack_bites_counters_from_choices(
	confines: &Confines,
	eligible: &[EligibleBitesPassage],
	choices: &[BitesCounterChoice],
) -> Result<PackedBitesCounters, FitError> {
	if eligible.is_empty() {
		return Err(FitError::TooSmall {
			reason: "bites counter passage",
		});
	}
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let faces: Vec<PlanOpeningFace> = eligible.iter().map(|e| e.face).collect();
	let specs: Vec<OptionalFaceBand> = choices
		.iter()
		.take(eligible.len())
		.map(|c| OptionalFaceBand {
			place: c.place,
			along: c.along,
			depth: c.depth,
			along_t: c.along_t,
		})
		.collect();
	let counters2 = pack_optional_face_bands(host, &faces, &specs);
	if counters2.is_empty() {
		return Err(FitError::TooSmall {
			reason: "bites counter passage",
		});
	}
	let counters: Vec<Aabb3d> = counters2
		.into_iter()
		.map(|c| plan_to_aabb3(&confines.bounds, c, PlanAxes::XZ))
		.collect();
	Ok(PackedBitesCounters {
		counters,
		passages: eligible.iter().map(|e| e.bounds).collect(),
		faces,
	})
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

/// Minimum shared border between seating and a passage's long opening face.
pub const BITES_SEATING_FACE_CONTACT: f32 = 1.0;

/// Sit-down regions: face seed → grow seating to `seating_area_target` → kitchen remainder.
///
/// Kitchen is packed **after** seating reaches its area target so a max-empty
/// kitchen seed cannot dominate the free volume.
pub fn pack_bites_sitdown_regions(
	bounds: &Aabb3d,
	counters: &[Aabb3d],
	faces: &[PlanOpeningFace],
	seating_area_target: f32,
	seating_contact: f32,
	seating_seed_depth: f32,
	seating_along_t: f32,
	min_plan: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let host = aabb3_to_plan(bounds, PlanAxes::XZ);
	let counter_plans: Vec<_> = counters
		.iter()
		.map(|c| aabb3_to_plan(c, PlanAxes::XZ))
		.collect();
	let contact = seating_contact.max(BITES_SEATING_FACE_CONTACT);
	let depth = seating_seed_depth.max(min_plan);

	let mut seating2 = None;
	let mut seating_face = None;
	let mut order: Vec<usize> = (0..faces.len()).collect();
	order.sort_by(|a, b| {
		faces[*b]
			.along_len()
			.partial_cmp(&faces[*a].along_len())
			.unwrap_or(std::cmp::Ordering::Equal)
	});
	for &i in &order {
		if let Some(seed) = seed_from_free_opening_face(
			host,
			faces[i],
			&counter_plans,
			contact,
			depth,
			seating_along_t,
		) {
			seating2 = Some(seed);
			seating_face = Some(faces[i]);
			break;
		}
		for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
			if let Some(seed) =
				seed_from_free_opening_face(host, faces[i], &counter_plans, contact, depth, t)
			{
				seating2 = Some(seed);
				seating_face = Some(faces[i]);
				break;
			}
		}
		if seating2.is_some() {
			break;
		}
	}
	let seating_seed = seating2?;
	let seating_face = seating_face?;

	// Block the outward side of the opening face so grow cannot peel seating
	// off the ≥1m long-face contact.
	let mut seating_hard = counter_plans.clone();
	seating_hard.push(outward_face_block(host, seating_face));

	let counter_area: f32 = counter_plans.iter().copied().map(aabb2_area).sum();
	let usable = (aabb2_area(host) - counter_area).max(0.0);
	// Leave at least a 1×1 kitchen opportunity when possible.
	let kitchen_reserve = (min_plan * min_plan).min(usable * 0.25);
	let target_s = seating_area_target
		.max(min_plan * min_plan)
		.min((usable - kitchen_reserve).max(min_plan * min_plan));

	// Grow seating first (no kitchen seed yet) so the area target is real.
	let seating2 = grow_aabb2_toward_area(host, seating_seed, &seating_hard, target_s);
	let seating2 = clamp_min_size2(seating2, Vec2::splat(min_plan))?;
	if shared_opening_border_len(seating2, seating_face) + 1e-3 < contact {
		return None;
	}
	let seating_aabb = plan_to_aabb3(bounds, seating2, PlanAxes::XZ);

	let kitchen_seed = pack_bites_kitchen(bounds, counters, &[seating_aabb], min_plan)?;
	let kitchen2 = aabb3_to_plan(&kitchen_seed, PlanAxes::XZ);
	let mut kitchen_hard: Vec<_> = counter_plans
		.iter()
		.copied()
		.map(|c| inflate_aabb2(c, BITES_KITCHEN_COUNTER_CLEARANCE))
		.collect();
	kitchen_hard.push(seating2);
	// Kitchen takes leftover scraps only; do not re-grow seating into them.
	let kitchen2 = grow_aabb2(host, kitchen2, &kitchen_hard);
	let kitchen2 = clamp_min_size2(kitchen2, Vec2::splat(min_plan))?;

	Some((
		plan_to_aabb3(bounds, seating2, PlanAxes::XZ),
		plan_to_aabb3(bounds, kitchen2, PlanAxes::XZ),
	))
}

/// Slab covering the host on the outward side of `face` (keeps seeds on the face).
fn outward_face_block(host: Aabb2d, face: PlanOpeningFace) -> Aabb2d {
	if face.thru_is_x {
		if face.inward_positive {
			Aabb2d {
				min: host.min,
				max: Vec2::new(face.thru, host.max.y),
			}
		} else {
			Aabb2d {
				min: Vec2::new(face.thru, host.min.y),
				max: host.max,
			}
		}
	} else if face.inward_positive {
		Aabb2d {
			min: host.min,
			max: Vec2::new(host.max.x, face.thru),
		}
	} else {
		Aabb2d {
			min: Vec2::new(host.min.x, face.thru),
			max: host.max,
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
