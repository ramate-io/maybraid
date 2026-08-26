//! Shared plan helpers for placing [`StraightStair`] runs.

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::{Stair, StairNode, StraightStair};
use richmond_building_components::Placement;

use crate::paneling::quad_panel::QuadPanel;
use super::tread_end::TreadEnd;

fn normalize_tops(mut tops: Vec<f32>) -> Vec<f32> {
	tops.retain(|y| y.is_finite());
	tops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	tops.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
	tops
}

pub(crate) const EPS: f32 = 1e-5;

pub(crate) fn xz(p: Vec3) -> Vec2 {
	Vec2::new(p.x, p.z)
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

/// Tread width / depth from the tighter opening half-extent.
pub(crate) fn tread_dims(opening_min: f32) -> (f32, f32) {
	let width = (opening_min * 0.4).clamp(0.35, 1.1);
	let depth = (width * 0.55).clamp(0.25, 0.45);
	(width, depth)
}

/// Linear run: first tread centered on `start_xz` at `base_y`, travel along `+X`.
pub(crate) fn place_straight_run(
	start_xz: Vec2,
	base_y: f32,
	travel: Vec2,
	width: f32,
	depth: f32,
	local_tops: Vec<f32>,
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

/// Square rest at a turn: at least the tread width, not a sliver.
pub(crate) fn landing_size(tread_width: f32) -> f32 {
	tread_width.max(0.65)
}

/// [`landing_size`] capped so a short well side still has room for a run.
fn landing_size_for(tread_width: f32, shortest_side: f32) -> f32 {
	let want = landing_size(tread_width);
	let cap = (shortest_side * 0.4).max(tread_width * 0.75);
	want.min(cap)
}

pub(crate) fn run_top_y(node: &StairNode) -> f32 {
	match &node.geometry {
		Stair::Straight(g) => node.placement.translation.y + g.height,
	}
}

/// Plan segment with walk-on \(Y\) at each end.
#[derive(Clone, Copy)]
pub(crate) struct PathSeg {
	pub start: Vec2,
	pub end: Vec2,
	pub y0: f32,
	pub y1: f32,
}

/// Straight runs along `segs`. At a yaw jump the incoming run stops short,
/// a rest fills the corner (full tread width × reserved gap, including the
/// outer rail), and the next run starts at that pad's far edge. Colinear
/// joints keep an incoming-only strip — extruding along outgoing there
/// collapses the quad.
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
			(len > EPS).then_some(len)
		})
		.fold(f32::MAX, f32::min);
	let pad = landing_size_for(width, if shortest.is_finite() { shortest } else { width });
	let min_run = pref_depth * 0.4;
	let mut stairs = Vec::new();
	let mut pads = Vec::new();
	let mut skip = 0.0_f32;

	for (i, seg) in segs.iter().enumerate() {
		let travel = seg.end - seg.start;
		let Some(dir) = normalize_xz(travel) else {
			continue;
		};
		let remaining = travel.length();
		let used = skip.min(remaining);
		let at_joint = i + 1 < segs.len();
		let leftover = (remaining - used).max(0.0);
		if leftover < EPS {
			skip = 0.0;
			continue;
		}
		let reserve = if !at_joint {
			0.0
		} else if leftover > min_run {
			pad.min(leftover * 0.45).min(leftover - min_run)
		} else {
			leftover * 0.35
		};
		let run_len = leftover - reserve;
		if run_len < EPS {
			skip = 0.0;
			continue;
		}
		let height = (seg.y1 - seg.y0).abs().max(StraightStair::DEFAULT_TREAD_HEIGHT);
		let n = tread_count(height, run_len, pref_depth).max(1);
		let going = run_len / n as f32;
		let first = seg.start + dir * (used + 0.5 * going);
		let Some(node) =
			place_straight_run(first, seg.y0.min(seg.y1), dir, width, going, uniform_tops(height, n))
		else {
			skip = 0.0;
			continue;
		};
		skip = 0.0;
		if at_joint {
			let y = run_top_y(&node);
			let next = segs.get(i + 1);
			let next_dir = next.and_then(|s| normalize_xz(s.end - s.start));
			let next_len = next.map(|s| (s.end - s.start).length()).unwrap_or(0.0);
			let turned = next_dir.filter(|nd| dir.dot(*nd).abs() <= 0.85);
			let far = pad_far(reserve, width, next_len, min_run);
			let pad_slab = match turned {
				Some(nd) => corner_square_slab(
					seg.end,
					dir,
					nd,
					reserve,
					far,
					width,
					y,
					style,
					thickness,
				),
				None => TreadEnd::from_straight(&node).landing_along(
					dir,
					y,
					style,
					thickness,
					reserve.max(pad * 0.5),
					pref_depth * 0.4,
				),
			};
			if let Some(pad_slab) = pad_slab {
				skip = if turned.is_some() { far } else { 0.0 };
				pads.push(pad_slab);
			}
		}
		stairs.push(node);
	}
	(stairs, pads)
}

/// Outgoing pad extent, locked to the next-run skip. Caps so a short next
/// side still has room for a tread.
fn pad_far(reserve: f32, tread_width: f32, next_len: f32, min_run: f32) -> f32 {
	let wanted = reserve.max(0.5 * tread_width);
	if next_len > min_run + EPS {
		wanted.min(next_len - min_run).max(reserve)
	} else {
		wanted.min(next_len.max(reserve))
	}
}

/// Corner rest in the incoming × outgoing frame, flush with the last leading
/// and the next first riser, covering the full tread width (outer rail included).
///
/// Local \(x\) along `incoming`, \(y\) along `outgoing`, origin at `joint`:
/// \(x \in [-\texttt{along}, w/2]\), \(y \in [-w/2, \texttt{far}]\).
/// `along` is the reserved incoming gap (flush with the last leading).
/// `far` is the outgoing skip (flush with the next first riser).
fn corner_square_slab(
	joint: Vec2,
	incoming: Vec2,
	outgoing: Vec2,
	along: f32,
	far: f32,
	tread_width: f32,
	y: f32,
	style: PanelStyle,
	thickness: f32,
) -> Option<QuadPanel> {
	let incoming = normalize_xz(incoming)?;
	let outgoing = normalize_xz(outgoing)?;
	let along = along.max(1e-4);
	let far = far.max(along);
	let half = 0.5 * tread_width.max(1e-4);
	let pt = |u: f32, v: f32| {
		let p = joint + incoming * u + outgoing * v;
		Vec3::new(p.x, y, p.y)
	};
	Some(QuadPanel::slab(
		style,
		pt(-along, -half),
		pt(half, -half),
		pt(-along, far),
		pt(half, far),
		thickness,
	))
}
