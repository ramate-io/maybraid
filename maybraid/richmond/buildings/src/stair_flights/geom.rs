//! Shared plan helpers for placing [`StraightStair`] runs and level rests.

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::{Stair, StairNode, StraightStair};
use richmond_building_components::Placement;

use super::tread_end::TreadEnd;
use crate::paneling::quad_panel::QuadPanel;

fn normalize_tops(mut tops: Vec<f32>) -> Vec<f32> {
	tops.retain(|y| y.is_finite());
	tops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	tops.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
	tops
}

pub(crate) const EPS: f32 = 1e-5;

/// |incoming · outgoing| above this is treated as colinear (same pad axis).
const COLINEAR_DOT: f32 = 0.85;

pub(crate) fn xz(p: Vec3) -> Vec2 {
	Vec2::new(p.x, p.z)
}

pub(crate) fn at_y(p: Vec2, y: f32) -> Vec3 {
	Vec3::new(p.x, y, p.y)
}

pub(crate) fn normalize_xz(v: Vec2) -> Option<Vec2> {
	let len = v.length();
	if len < EPS {
		None
	} else {
		Some(v / len)
	}
}

/// Local \(+X\) in XZ for a placement yaw: \((\cosθ,\ -\sinθ)\).
pub(crate) fn travel_xz(yaw: f32) -> Vec2 {
	let (s, c) = yaw.sin_cos();
	Vec2::new(c, -s)
}

/// Yaw that sends local \(+X\) along `travel`.
pub(crate) fn yaw_of(travel: Vec2) -> f32 {
	(-travel.y).atan2(travel.x)
}

/// Default tread span as a fraction of the tighter opening half-extent.
pub(crate) const TREAD_FILL_DEFAULT: f32 = 0.4;
pub(crate) const TREAD_FILL_MIN: f32 = 0.2;
pub(crate) const TREAD_FILL_MAX: f32 = 0.95;
const TREAD_WIDTH_MIN_M: f32 = 0.35;

/// Default lapping ratio. Stay near this for one circuit on a ~3 m well.
pub(crate) const LAPPING_RATIO_DEFAULT: f32 = 0.55;
pub(crate) const LAPPING_RATIO_MIN: f32 = 0.2;
pub(crate) const LAPPING_RATIO_MAX: f32 = 2.0;
const TREAD_DEPTH_MIN_M: f32 = 0.15;
const TREAD_DEPTH_MAX_M: f32 = 3.0;

/// Keep an authored fill inside the legal band.
pub(crate) fn clamp_tread_fill(fill: f32) -> f32 {
	if !fill.is_finite() {
		TREAD_FILL_DEFAULT
	} else {
		fill.clamp(TREAD_FILL_MIN, TREAD_FILL_MAX)
	}
}

/// Keep an authored lapping ratio inside \(0.2\ldots 2.0\).
pub(crate) fn clamp_lapping_ratio(ratio: f32) -> f32 {
	if !ratio.is_finite() {
		LAPPING_RATIO_DEFAULT
	} else {
		ratio.clamp(LAPPING_RATIO_MIN, LAPPING_RATIO_MAX)
	}
}

/// Tread width / preferred going from opening half-extent, fill, and lapping ratio.
///
/// Preferred going is a wish: rectangular-spiral may add circuits so rise stays
/// near [`StraightStair::DEFAULT_TREAD_HEIGHT`] (0.18 m). Values ≳ 1 on a ~3 m
/// well usually stack another lap.
pub(crate) fn tread_dims(opening_min: f32, fill: f32, lapping_ratio: f32) -> (f32, f32) {
	let fill = clamp_tread_fill(fill);
	let lapping_ratio = clamp_lapping_ratio(lapping_ratio);
	let opening_min = opening_min.max(1e-4);
	let width = (opening_min * fill).clamp(TREAD_WIDTH_MIN_M, opening_min * TREAD_FILL_MAX);
	let depth = (width * lapping_ratio).clamp(TREAD_DEPTH_MIN_M, TREAD_DEPTH_MAX_M);
	(width, depth)
}

/// Linear run: first tread centered on `start_xz` at `base_y`, travel along \(+X\).
///
/// `flush_start` packs the first kit's \(X \to -2\) bleed into the going so the
/// mesh trailing sits on the walkable trailing (landing / walk-on edge).
pub(crate) fn place_straight_run(
	start_xz: Vec2,
	base_y: f32,
	travel: Vec2,
	width: f32,
	depth: f32,
	local_tops: Vec<f32>,
	flush_start: bool,
) -> Option<StairNode> {
	let travel = normalize_xz(travel)?;
	let tops = normalize_tops(local_tops);
	if tops.is_empty() {
		return None;
	}
	let n = tops.len() as f32;
	let depth = depth.max(1e-4);
	let height = tops.last().copied().unwrap_or(StraightStair::DEFAULT_TREAD_HEIGHT);
	let geometry = Stair::Straight(StraightStair {
		height,
		length: n * depth,
		width: width.max(1e-4),
		depth,
		tread_height: StraightStair::DEFAULT_TREAD_HEIGHT,
		tread_tops: tops,
		flush_start,
	});
	Some(StairNode::rough_stone(
		geometry,
		Placement::new(Vec3::new(start_xz.x, base_y, start_xz.y), yaw_of(travel)),
	))
}

/// Uniform local tops for a run of `n` treads rising `height`.
pub(crate) fn uniform_tops(height: f32, n: u32) -> Vec<f32> {
	let n = n.max(1);
	let rise = height.max(1e-4) / n as f32;
	(1..=n).map(|i| i as f32 * rise).collect()
}

/// Tread count that covers `height` at the default rise and `length` at `depth`.
pub(crate) fn tread_count(height: f32, length: f32, depth: f32) -> u32 {
	let from_rise = (height.max(1e-4) / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0);
	let from_going = (length.max(1e-4) / depth.max(1e-4)).floor().max(1.0);
	from_rise.max(from_going) as u32
}

pub(crate) fn run_top_y(node: &StairNode) -> f32 {
	match &node.geometry {
		Stair::Straight(g) => node.placement.translation.y + g.height,
	}
}

/// Level XZ rectangle; kit top sits on `y` ([`QuadPanel::slab`]).
pub(crate) fn level_rect(
	style: PanelStyle,
	a0: Vec2,
	a1: Vec2,
	b0: Vec2,
	b1: Vec2,
	y: f32,
	thickness: f32,
) -> QuadPanel {
	QuadPanel::slab(style, at_y(a0, y), at_y(a1, y), at_y(b0, y), at_y(b1, y), thickness)
}

/// Plan segment with walk-on \(Y\) at each end.
#[derive(Clone, Copy)]
pub(crate) struct PathSeg {
	pub start: Vec2,
	pub end: Vec2,
	pub y0: f32,
	pub y1: f32,
}

/// One joint: incoming rest, outgoing skip, and the run between them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JointBudget {
	pub along: f32,
	pub far: f32,
	pub run_len: f32,
	/// 180° rest: pad stops at the inner edge; the return run starts there.
	pub u_turn: bool,
}

impl JointBudget {
	/// `wanted` rest, never eating `min_tread` off this side or the next.
	pub fn new(
		available: f32,
		next_len: f32,
		wanted: f32,
		min_tread: f32,
		last_side: bool,
		turned: bool,
	) -> Self {
		if last_side {
			return Self { along: 0.0, far: 0.0, run_len: available.max(0.0), u_turn: false };
		}
		let available = available.max(0.0);
		let mut along = if available > min_tread + EPS {
			wanted.min(available - min_tread)
		} else {
			available * 0.35
		};
		along = along.max(0.0);
		let mut run_len = (available - along).max(0.0);
		if run_len < EPS {
			along = 0.0;
			run_len = available;
		}
		let far = if turned {
			if next_len > min_tread + EPS {
				wanted.min(next_len - min_tread).max(0.0)
			} else {
				(next_len * 0.35).max(0.0)
			}
		} else {
			0.0
		};
		Self { along, far, run_len, u_turn: false }
	}
}

/// Rest target: at least ~tread width, at most 40% of the shortest side.
pub(crate) fn wanted_rest(tread_width: f32, shortest_side: f32) -> f32 {
	let want = tread_width.max(0.65);
	let cap = (shortest_side * 0.4).max(tread_width * 0.75);
	want.min(cap)
}

/// First trailing of the next run: inner edge on a U / 180°, outgoing far on an L.
fn departing_riser(end: TreadEnd, outgoing: Option<Vec2>, budget: JointBudget) -> Vec2 {
	let incoming = normalize_xz(end.travel).unwrap_or(Vec2::X);
	if budget.u_turn {
		return end.leading_mid();
	}
	if let Some(out) = outgoing.and_then(normalize_xz) {
		if is_yaw_joint(incoming, out) {
			return end.leading_mid() + incoming * budget.along + out * budget.far;
		}
	}
	end.leading_mid()
}

/// Straight runs along `segs`. One [`JointBudget`] per joint; the next first
/// riser is the authored pad edge, not a polyline skip.
pub(crate) fn place_runs_with_corner_landings(
	segs: &[PathSeg],
	width: f32,
	pref_depth: f32,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Vec<QuadPanel>) {
	let shortest = segs
		.iter()
		.filter_map(|s| {
			let len = (s.end - s.start).length();
			(len > width * 1.15).then_some(len)
		})
		.fold(f32::MAX, f32::min);
	let wanted = wanted_rest(width, if shortest.is_finite() { shortest } else { width });
	let min_tread = pref_depth * 0.4;
	let mut stairs = Vec::new();
	let mut pads = Vec::new();
	let mut pending_riser: Option<(Vec2, bool)> = None;

	for (i, seg) in segs.iter().enumerate() {
		let travel = seg.end - seg.start;
		let Some(dir) = normalize_xz(travel) else {
			continue;
		};
		let remaining = travel.length();
		if (seg.y1 - seg.y0).abs() < EPS {
			continue;
		}
		let used = if let Some((edge, skip_short)) = pending_riser {
			let inset = (edge - seg.start).dot(dir).max(0.0);
			if (skip_short && remaining <= width * 1.15) || inset + EPS >= remaining {
				continue;
			}
			inset
		} else {
			0.0
		};
		let leftover = (remaining - used).max(0.0);
		if leftover < EPS {
			continue;
		}
		let at_joint = i + 1 < segs.len();
		let next = segs.get(i + 1);
		let next_dir = next.and_then(|s| normalize_xz(s.end - s.start));
		let next_len = next.map(|s| (s.end - s.start).length()).unwrap_or(0.0);
		let bridge = at_joint
			&& next_len <= width * 1.15
			&& leftover > width * 2.0
			&& next_len + EPS < leftover;
		let turned = next_dir.is_some_and(|nd| is_yaw_joint(dir, nd)) || bridge;
		let mut budget = JointBudget::new(leftover, next_len, wanted, min_tread, !at_joint, turned);
		if bridge {
			// Pad must cover the return's far rail (centerline + half width).
			budget.far = next_len + 0.5 * width;
			budget.u_turn = true;
		}
		if budget.run_len < EPS {
			continue;
		}
		let y0 = stairs.last().map(run_top_y).unwrap_or(seg.y0.min(seg.y1));
		let height = (seg.y1 - seg.y0).abs().max(StraightStair::DEFAULT_TREAD_HEIGHT);
		let n = tread_count(height, budget.run_len, pref_depth).max(1);
		let going = budget.run_len / n as f32;
		let first = seg.start + dir * (used + 0.5 * going);
		let Some(node) =
			place_straight_run(first, y0, dir, width, going, uniform_tops(height, n), true)
		else {
			continue;
		};
		pending_riser = None;
		if at_joint {
			let end = TreadEnd::from_straight(&node);
			if let Some(pad) = end.pad(next_dir, budget, width, run_top_y(&node), style, thickness)
			{
				pending_riser = Some((departing_riser(end, next_dir, budget), budget.u_turn));
				pads.push(pad);
			}
		}
		stairs.push(node);
	}
	(stairs, pads)
}

/// Incoming × outgoing (or incoming × across) rest from a last leading.
pub(crate) fn is_yaw_joint(incoming: Vec2, outgoing: Vec2) -> bool {
	incoming.dot(outgoing).abs() <= COLINEAR_DOT
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn budget_leaves_min_tread_on_both_sides() {
		let b = JointBudget::new(1.0, 1.0, 0.65, 0.1, false, true);
		assert!((b.along - 0.65).abs() < 1e-4);
		assert!((b.run_len - 0.35).abs() < 1e-4);
		assert!((b.far - 0.65).abs() < 1e-4);
		assert!((b.along + b.run_len - 1.0).abs() < 1e-4);
	}

	#[test]
	fn budget_shrinks_rest_on_a_short_side() {
		let b = JointBudget::new(0.28, 0.28, 0.65, 0.1, false, true);
		assert!(b.run_len + 1e-4 >= 0.1, "run={:?}", b);
		assert!(b.far <= 0.18 + 1e-4, "far must leave min_tread on the next side, {b:?}");
		assert!(b.along + b.run_len <= 0.28 + 1e-4, "{b:?}");
		assert!(b.run_len >= EPS, "never drop the side, {b:?}");
	}

	#[test]
	fn last_side_has_no_rest() {
		let b = JointBudget::new(1.0, 0.0, 0.65, 0.1, true, false);
		assert_eq!(b.along, 0.0);
		assert_eq!(b.far, 0.0);
		assert!((b.run_len - 1.0).abs() < 1e-4);
	}

	#[test]
	fn tread_fill_scales_width_relative_to_the_opening() {
		let (narrow, _) = tread_dims(1.2, 0.4, LAPPING_RATIO_DEFAULT);
		let (wide, _) = tread_dims(1.2, 0.8, LAPPING_RATIO_DEFAULT);
		assert!((narrow - 0.48).abs() < 1e-4);
		assert!((wide - 0.96).abs() < 1e-4);
	}

	#[test]
	fn tread_fill_clamps_and_stays_inside_the_opening() {
		assert!((clamp_tread_fill(3.0) - TREAD_FILL_MAX).abs() < 1e-4);
		assert!((clamp_tread_fill(0.0) - TREAD_FILL_MIN).abs() < 1e-4);
		let (w, _) = tread_dims(3.0, 0.4, LAPPING_RATIO_DEFAULT);
		assert!(
			(w - 1.2).abs() < 1e-4,
			"large wells follow the fraction, not a 1.1 m cap, got {w}"
		);
		let (tiny, _) = tread_dims(0.45, 0.4, LAPPING_RATIO_DEFAULT);
		assert!((tiny - TREAD_WIDTH_MIN_M).abs() < 1e-4);
		let (capped, _) = tread_dims(1.2, 0.95, LAPPING_RATIO_DEFAULT);
		assert!((capped - 1.2 * TREAD_FILL_MAX).abs() < 1e-4);
	}

	#[test]
	fn lapping_ratio_scales_preferred_going() {
		let (_, shallow) = tread_dims(1.2, 0.6, 0.4);
		let (_, deep) = tread_dims(1.2, 0.6, 0.7);
		assert!((shallow - 0.288).abs() < 1e-4, "got {shallow}");
		assert!((deep - 0.504).abs() < 1e-4, "0.72*0.7 should not hit a comfort cap, got {deep}");
		let (_, chunky) = tread_dims(1.2, 0.4, 2.0);
		assert!((chunky - 0.96).abs() < 1e-4, "lapping_ratio 2.0 on a 0.48 m tread, got {chunky}");
	}

	#[test]
	fn lapping_ratio_clamps() {
		assert!((clamp_lapping_ratio(10.0) - LAPPING_RATIO_MAX).abs() < 1e-4);
		assert!((clamp_lapping_ratio(0.0) - LAPPING_RATIO_MIN).abs() < 1e-4);
	}
}
