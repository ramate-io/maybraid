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

/// Straight runs along `segs`. At each interior joint the incoming run stops
/// short and a **rectangle** landing is extruded along incoming travel (the
/// leading edge is perpendicular to travel, so this stays planar). The next
/// run starts at the joint — the landing already fills up to that point.
pub(crate) fn place_runs_with_corner_landings(
	segs: &[PathSeg],
	width: f32,
	pref_depth: f32,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Vec<QuadPanel>) {
	let pad = landing_size(width);
	let mut stairs = Vec::new();
	let mut pads = Vec::new();

	for (i, seg) in segs.iter().enumerate() {
		let travel = seg.end - seg.start;
		let Some(dir) = normalize_xz(travel) else {
			continue;
		};
		let remaining = travel.length();
		let at_joint = i + 1 < segs.len();
		let reserve = if at_joint { pad.min(remaining * 0.45) } else { 0.0 };
		let run_len = remaining - reserve;
		if run_len < pref_depth * 0.55 {
			continue;
		}
		let height = (seg.y1 - seg.y0).abs().max(StraightStair::DEFAULT_TREAD_HEIGHT);
		let n = tread_count(height, run_len, pref_depth);
		let going = run_len / n as f32;
		let first = seg.start + dir * (0.5 * going);
		let Some(node) =
			place_straight_run(first, seg.y0.min(seg.y1), dir, width, going, uniform_tops(height, n))
		else {
			continue;
		};
		if at_joint {
			if let Some(pad_slab) = TreadEnd::from_straight(&node).landing_along(
				dir,
				run_top_y(&node),
				style,
				thickness,
				reserve.max(pad * 0.5),
				pref_depth * 0.4,
			) {
				pads.push(pad_slab);
			}
		}
		stairs.push(node);
	}
	(stairs, pads)
}
