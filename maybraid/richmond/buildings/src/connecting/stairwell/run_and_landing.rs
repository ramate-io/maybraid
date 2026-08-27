//! Run-and-landing flight inside an exclusive [`WellAabb`].
//!
//! Every flight is an **I**: hug the wall, fill one half so the other half can
//! tile. Extra laps for going add an interior landing and run back the other
//! half when [`super::spiral::MIN_GOING`] and [`super::spiral::MIN_HEADROOM`]
//! allow. If the last I would still need a U to the walk-off, try **one**
//! routing switchback first. Interior turnarounds are flight-width deep so a
//! 180° step is walkable. L/U leftover is the unused half (not a ribbon).

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::placed::Placement;
use richmond_building_components::stairs::{Stair, StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;

use super::spiral::{MIN_GOING, MIN_HEADROOM};
use super::well::WellAabb;
use super::WellSide;

const MIN_RUN: f32 = 0.08;
const MIN_LANDING: f32 = 0.12;

/// Which half of the walk-on split the I occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Half {
	/// Toward [`WellSide::ccw_next`] of the walk-on (first I).
	Ccw,
	Cw,
}

impl Half {
	fn other(self) -> Self {
		match self {
			Self::Ccw => Self::Cw,
			Self::Cw => Self::Ccw,
		}
	}

	fn hug(self, walk_on: WellSide) -> WellSide {
		match self {
			Self::Ccw => walk_on.ccw_next(),
			Self::Cw => walk_on.cw_next(),
		}
	}
}

struct Flight {
	half: Half,
	travel: Vec2,
	start_side: WellSide,
	end_side: WellSide,
	k: u32,
	start_pad: f32,
	end_pad: f32,
}

/// Half-well I flights + interior turnarounds + walk-off strip path.
pub(crate) fn fit(
	well: &WellAabb,
	style: PanelStyle,
	thickness: f32,
	want_landing: bool,
) -> (Vec<StairNode>, Option<QuadPanel>, Vec<QuadPanel>) {
	let rise = well.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let width = i_width(well);
	let n = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32;
	let door = door_pad_m(well);
	let turn = turn_pad_m(well);
	let along = i_along(well);
	let run = (along - door - turn).max(MIN_RUN);
	let laps = route_lap(lap_count(n, run, rise), n, well.walk_on, well.walk_off);
	let counts = split_treads(n, laps);
	let flights = flights(well, &counts, door, turn);
	let rise_step = rise / n as f32;
	let mut y = well.bottom_y();
	let mut stairs = Vec::new();
	let mut mids = Vec::new();
	for (i, flight) in flights.iter().enumerate() {
		if flight.k == 0 {
			continue;
		}
		let run = (along - flight.start_pad - flight.end_pad).max(MIN_RUN);
		let going = (run / flight.k as f32).max(1e-4);
		stairs.push(flight_node(well, flight, width, going, rise_step, y));
		y += flight.k as f32 * rise_step;
		let last = i + 1 == flights.len();
		if !last {
			mids.push(wall_strip(well, flight.end_side, y, flight.end_pad, style, thickness));
		}
	}
	let landing = if want_landing {
		if let Some(last) = flights.iter().rev().find(|f| f.k > 0) {
			let (door, extras) = walk_off_pads(well, last, door, style, thickness);
			mids.extend(extras);
			Some(door)
		} else {
			Some(well.walk_off_landing_strip(style, thickness, door))
		}
	} else {
		None
	};
	(stairs, landing, mids)
}

fn i_width(well: &WellAabb) -> f32 {
	match well.walk_on {
		WellSide::NegX | WellSide::PosX => well.half_z(),
		WellSide::NegZ | WellSide::PosZ => well.half_x(),
	}
	.max(1e-4)
}

fn i_along(well: &WellAabb) -> f32 {
	match well.walk_on {
		WellSide::NegX | WellSide::PosX => 2.0 * well.half_x(),
		WellSide::NegZ | WellSide::PosZ => 2.0 * well.half_z(),
	}
}

fn door_pad_m(well: &WellAabb) -> f32 {
	MIN_GOING.min(i_along(well) * 0.2).max(MIN_LANDING)
}

/// Turnaround depth: as deep as the I is wide, so a 180° step is walkable.
fn turn_pad_m(well: &WellAabb) -> f32 {
	let door = door_pad_m(well);
	let cap = (i_along(well) - door - MIN_RUN).max(door);
	i_width(well).max(door).min(cap)
}

fn lap_count(n: u32, run: f32, rise: f32) -> u32 {
	let mut laps = 1u32;
	let going_of = |laps: u32| (laps as f32 * run) / n.max(1) as f32;
	while going_of(laps) + 1e-4 < MIN_GOING {
		let next = laps + 1;
		if rise / next as f32 + 1e-4 < MIN_HEADROOM {
			break;
		}
		laps = next;
	}
	laps
}

/// One extra I when the last flight would finish opposite the walk-off (a U).
fn route_lap(laps: u32, n: u32, walk_on: WellSide, walk_off: WellSide) -> u32 {
	let end = if laps % 2 == 1 { walk_on.opposite() } else { walk_on };
	if end.opposite() == walk_off && n > laps {
		laps + 1
	} else {
		laps
	}
}

fn split_treads(n: u32, laps: u32) -> Vec<u32> {
	let laps = laps.max(1);
	let base = n / laps;
	let rem = n % laps;
	(0..laps).map(|i| base + u32::from(i < rem)).collect()
}

fn flights(well: &WellAabb, counts: &[u32], door: f32, turn: f32) -> Vec<Flight> {
	let inward = -well.walk_on.into_xz();
	let far = well.walk_on.opposite();
	let n = counts.len();
	counts
		.iter()
		.enumerate()
		.map(|(i, &k)| {
			let outbound = i % 2 == 0;
			Flight {
				half: if outbound { Half::Ccw } else { Half::Cw },
				travel: if outbound { inward } else { -inward },
				start_side: if outbound { well.walk_on } else { far },
				end_side: if outbound { far } else { well.walk_on },
				k,
				start_pad: if i == 0 { door } else { turn },
				end_pad: if i + 1 == n { door } else { turn },
			}
		})
		.collect()
}

/// Strip path from the last I's end wall to walk-off, avoiding a lid on the I.
fn connect_sides(end: WellSide, off: WellSide, avoid_hug: WellSide) -> Vec<WellSide> {
	if end == off {
		return vec![end];
	}
	let ccw = walk(end, off, WellSide::ccw_next);
	let cw = walk(end, off, WellSide::cw_next);
	let hits = |path: &[WellSide]| path.iter().skip(1).any(|&s| s == avoid_hug && s != off);
	match (hits(&ccw), hits(&cw)) {
		(true, false) => cw,
		(false, true) => ccw,
		_ if ccw.len() <= cw.len() => ccw,
		_ => cw,
	}
}

fn walk(start: WellSide, end: WellSide, step: fn(WellSide) -> WellSide) -> Vec<WellSide> {
	let mut path = vec![start];
	let mut s = start;
	while s != end {
		s = step(s);
		path.push(s);
		if path.len() > 4 {
			break;
		}
	}
	path
}

fn flight_node(
	well: &WellAabb,
	flight: &Flight,
	width: f32,
	going: f32,
	rise_step: f32,
	y: f32,
) -> StairNode {
	let start = first_tread_xz(well, flight, going);
	let length = flight.k as f32 * going;
	let height = flight.k as f32 * rise_step;
	let mut geom = StraightStair::run(height, length, width, going).with_flush_start(true);
	geom.tread_height = rise_step.max(1e-4);
	StairNode::rough_stone(
		Stair::Straight(geom),
		Placement::new(Vec3::new(start.x, y, start.y), yaw_of(flight.travel)),
	)
}

fn first_tread_xz(well: &WellAabb, flight: &Flight, going: f32) -> Vec2 {
	let lateral = half_center(well, flight.half);
	let start_mid = well.side_mid(flight.start_side, 0.0);
	let start_edge =
		Vec2::new(start_mid.x, start_mid.z) - flight.start_side.into_xz() * flight.start_pad;
	let t = unit(flight.travel);
	let n = Vec2::new(-t.y, t.x);
	let p = start_edge + t * (0.5 * going);
	t * p.dot(t) + n * lateral.dot(n)
}

fn half_center(well: &WellAabb, half: Half) -> Vec2 {
	let c = well.center_xz();
	let hug = half.hug(well.walk_on).into_xz();
	let ext = if hug.x.abs() > hug.y.abs() { well.half_x() } else { well.half_z() };
	c + hug * (0.5 * ext)
}

fn yaw_of(travel: Vec2) -> f32 {
	let t = unit(travel);
	(-t.y).atan2(t.x)
}

fn unit(v: Vec2) -> Vec2 {
	if v.length_squared() < 1e-8 {
		Vec2::Y
	} else {
		v.normalize()
	}
}

fn walk_off_pads(
	well: &WellAabb,
	last: &Flight,
	pad: f32,
	style: PanelStyle,
	thickness: f32,
) -> (QuadPanel, Vec<QuadPanel>) {
	if last.end_side == well.walk_off {
		return (door_slab(well, last, pad, style, thickness), Vec::new());
	}
	let mut extras = vec![wall_strip(well, last.end_side, well.top_y(), pad, style, thickness)];
	let unused = last.half.other();
	let fat = half_slab(well, unused, well.top_y(), style, thickness);
	if well.walk_off != last.half.hug(well.walk_on) {
		(fat, extras)
	} else {
		extras.push(fat);
		(door_slab(well, last, pad, style, thickness), extras)
	}
}

fn door_slab(
	well: &WellAabb,
	last: &Flight,
	pad: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let hug = last.half.hug(well.walk_on);
	if well.walk_off == hug {
		wall_half_strip(well, well.walk_off, last.end_side, well.top_y(), pad, style, thickness)
	} else {
		well.walk_off_landing_strip(style, thickness, pad)
	}
}

fn half_slab(well: &WellAabb, half: Half, y: f32, style: PanelStyle, thickness: f32) -> QuadPanel {
	let (a, b) = half_bounds_xz(well, half);
	QuadPanel::slab(
		style,
		Vec3::new(a.x, y, a.y),
		Vec3::new(b.x, y, a.y),
		Vec3::new(a.x, y, b.y),
		Vec3::new(b.x, y, b.y),
		thickness,
	)
}

fn half_bounds_xz(well: &WellAabb, half: Half) -> (Vec2, Vec2) {
	let min = well.min();
	let max = well.max();
	let c = well.center_xz();
	match half.hug(well.walk_on) {
		WellSide::PosX => (Vec2::new(c.x, min.z), Vec2::new(max.x, max.z)),
		WellSide::NegX => (Vec2::new(min.x, min.z), Vec2::new(c.x, max.z)),
		WellSide::PosZ => (Vec2::new(min.x, c.y), Vec2::new(max.x, max.z)),
		WellSide::NegZ => (Vec2::new(min.x, min.z), Vec2::new(max.x, c.y)),
	}
}

fn wall_strip(
	well: &WellAabb,
	side: WellSide,
	y: f32,
	depth: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let (lo, hi) = face_along(well, side);
	strip_slab(well, side, y, depth, lo, hi, style, thickness)
}

fn wall_half_strip(
	well: &WellAabb,
	side: WellSide,
	toward: WellSide,
	y: f32,
	depth: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let (lo, hi) = half_along(well, side, toward);
	strip_slab(well, side, y, depth, lo, hi, style, thickness)
}

fn face_along(well: &WellAabb, side: WellSide) -> (f32, f32) {
	match side {
		WellSide::NegZ | WellSide::PosZ => (well.min().x, well.max().x),
		WellSide::NegX | WellSide::PosX => (well.min().z, well.max().z),
	}
}

fn half_along(well: &WellAabb, side: WellSide, toward: WellSide) -> (f32, f32) {
	let (lo, hi) = face_along(well, side);
	let mid = 0.5 * (lo + hi);
	let toward_xz = toward.into_xz();
	let sign = match side {
		WellSide::NegZ | WellSide::PosZ => toward_xz.x,
		WellSide::NegX | WellSide::PosX => toward_xz.y,
	};
	if sign >= 0.0 {
		(mid, hi)
	} else {
		(lo, mid)
	}
}

fn strip_slab(
	well: &WellAabb,
	side: WellSide,
	y: f32,
	depth: f32,
	along0: f32,
	along1: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let d = depth.max(MIN_LANDING);
	let lo = along0.min(along1);
	let hi = along0.max(along1);
	let min = well.min();
	let max = well.max();
	let (a0, a1, b0, b1) = match side {
		WellSide::NegZ => (
			Vec3::new(lo, y, min.z),
			Vec3::new(hi, y, min.z),
			Vec3::new(lo, y, min.z + d),
			Vec3::new(hi, y, min.z + d),
		),
		WellSide::PosZ => (
			Vec3::new(lo, y, max.z),
			Vec3::new(hi, y, max.z),
			Vec3::new(lo, y, max.z - d),
			Vec3::new(hi, y, max.z - d),
		),
		WellSide::NegX => (
			Vec3::new(min.x, y, lo),
			Vec3::new(min.x, y, hi),
			Vec3::new(min.x + d, y, lo),
			Vec3::new(min.x + d, y, hi),
		),
		WellSide::PosX => (
			Vec3::new(max.x, y, lo),
			Vec3::new(max.x, y, hi),
			Vec3::new(max.x - d, y, lo),
			Vec3::new(max.x - d, y, hi),
		),
	};
	QuadPanel::slab(style, a0, a1, b0, b1, thickness)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn same_side_is_a_u_around_the_unused_half() {
		let path = connect_sides(WellSide::PosZ, WellSide::NegZ, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::NegX, WellSide::NegZ]);
	}

	#[test]
	fn unused_adjacent_is_an_l() {
		let path = connect_sides(WellSide::PosZ, WellSide::NegX, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::NegX]);
	}

	#[test]
	fn hug_adjacent_is_an_l_on_the_end_corner() {
		let path = connect_sides(WellSide::PosZ, WellSide::PosX, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::PosX]);
	}

	#[test]
	fn already_on_the_walk_off_wall_is_just_the_door() {
		let path = connect_sides(WellSide::PosZ, WellSide::PosZ, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ]);
	}

	#[test]
	fn return_i_same_side_is_just_the_door() {
		let path = connect_sides(WellSide::NegZ, WellSide::NegZ, WellSide::NegX);
		assert_eq!(path, vec![WellSide::NegZ]);
	}

	#[test]
	fn u_situation_adds_one_routing_lap() {
		assert_eq!(route_lap(1, 17, WellSide::NegZ, WellSide::NegZ), 2);
		assert_eq!(route_lap(1, 17, WellSide::NegZ, WellSide::PosZ), 1);
		assert_eq!(route_lap(1, 1, WellSide::NegZ, WellSide::NegZ), 1);
	}
}
