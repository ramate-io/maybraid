//! I / L / U flights: openings own the ends; extra lapping is a stacked switchback.
//!
//! Walk-on sides pick the graph. Adjacent → compact L. Same side → U (two
//! parallel runs, far landing spans both). Opposite or a short rise → I.
//! Extra lapping reuses those two corridors — same plan silhouette, stacked
//! in \(Y\) — instead of packing new laterals or reversing on the last
//! column. After the last zig-zag, an L arrives with a landing to the
//! walk-on, not another run of stairs. A long L between offset openings
//! stays the polyline path.

use bevy_math::Vec2;
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::{StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::composed::ComposedFlight;
use crate::stair_flights::geom::{
	clamp_lapping_ratio, is_lateral, normalize_xz, place_runs_with_corner_landings, run_top_y, xz,
	JointBudget, PathSeg, EPS,
};
use crate::stair_flights::{FlightPolyline, TreadEnd, WellFit};

/// Plan points closer than this are the same waypoint.
const DEDUP_M: f32 = 0.12;
/// Walk-on is on this side of the well if |u| or |u − far| is below this.
const SIDE_ALIGN_M: f32 = 0.35;
const IN_WELL_SLACK_M: f32 = 0.3;
const ADJACENT_RIM_M: f32 = 0.4;
/// Compact I / L is enough when needed going fits this × the path length.
const COMPACT_FIT: f32 = 1.05;
const STATION_M: f32 = 0.15;
/// Walk-on this close to the last leading is already on the rim.
const LATERAL_ARRIVE: f32 = 0.75;
/// Gap between the two switchback corridors.
const CORRIDOR_GAP_M: f32 = 0.12;

/// Fit an I / L / U path, or the polyline L when the arrive is outside the well.
pub fn fit(
	polyline: FlightPolyline,
	fit: WellFit,
	style: PanelStyle,
	slab_thickness: f32,
) -> ComposedFlight {
	let (stairs, pads) = fit_runs(&polyline, fit, style, slab_thickness);
	ComposedFlight::new(polyline, stairs, pads)
}

fn fit_runs(
	polyline: &FlightPolyline,
	fit: WellFit,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Vec<QuadPanel>) {
	let (_, fill_depth) = fit.tread_dims();
	let (width, pref_depth) = switchback_dims(fit);
	let rise = polyline.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let path = plan_path(polyline, fit, width, fill_depth);
	let segs = segs_along_path(&path, fit.lower_walk_on.y, rise, width);
	let (stairs, mut pads) =
		place_runs_with_corner_landings(&segs, width, pref_depth, style, thickness, fill_depth);
	if let Some(pad) = arrive_landing(&stairs, xz(fit.upper_walk_on), width, style, thickness) {
		pads.push(pad);
	}
	(stairs, pads)
}

/// Two flights fill the well (minus a gap), not the spiral fill fraction.
fn switchback_dims(fit: WellFit) -> (f32, f32) {
	let (fill_w, _) = fit.tread_dims();
	let well = 2.0 * fit.half_width();
	let each = ((well - CORRIDOR_GAP_M) * 0.5).clamp(fill_w, well * 0.48);
	let depth = (each * clamp_lapping_ratio(fit.lapping_ratio)).clamp(0.15, 3.0);
	(each, depth)
}

/// Last zig-zag stops at the walk-on depth; this rectangle reaches the rim.
fn arrive_landing(
	stairs: &[StairNode],
	walk_on: Vec2,
	width: f32,
	style: PanelStyle,
	thickness: f32,
) -> Option<QuadPanel> {
	let end = TreadEnd::from_last_straight(stairs)?;
	let mid = end.leading_mid();
	let to = walk_on - mid;
	if to.length() <= width * LATERAL_ARRIVE {
		return None;
	}
	let incoming = normalize_xz(end.travel)?;
	let across = Vec2::new(-incoming.y, incoming.x);
	let side = to.dot(across);
	if side.abs() <= width * LATERAL_ARRIVE {
		return None;
	}
	let outgoing = across * side.signum();
	let along = to.dot(incoming).max(0.0);
	if along > width * 0.75 {
		return None;
	}
	let y = stairs.last().map(run_top_y)?;
	end.pad(
		Some(outgoing),
		JointBudget { along: 0.0, far: side.abs() + 0.5 * width, run_len: 0.0, u_turn: false },
		width,
		y,
		style,
		thickness,
	)
}

#[derive(Clone, Copy)]
struct Frame {
	start: Vec2,
	out: Vec2,
	right: Vec2,
	hw: f32,
	hd: f32,
	u_a: f32,
	v_a: f32,
}

impl Frame {
	fn new(fit: WellFit) -> Option<Self> {
		let out = normalize_xz(fit.lower_out)?;
		let start = xz(fit.lower_walk_on);
		let arrive = xz(fit.upper_walk_on);
		let delta = arrive - start;
		let right = Vec2::new(-out.y, out.x);
		let hw = fit.half_width();
		let hd = fit.half_depth();
		Some(Self { start, out, right, hw, hd, u_a: delta.dot(out), v_a: delta.dot(right) })
	}

	fn far(self) -> f32 {
		(2.0 * self.hd).max(0.5)
	}

	fn world(self, u: f32, v: f32) -> Vec2 {
		self.start + self.out * u + self.right * v
	}

	fn in_well(self) -> bool {
		let u_ok = self.u_a >= -IN_WELL_SLACK_M && self.u_a <= self.far() + IN_WELL_SLACK_M;
		let v_ok = self.v_a.abs() <= self.hw + IN_WELL_SLACK_M;
		u_ok && v_ok
	}

	fn same_side(self) -> bool {
		self.u_a.abs() < SIDE_ALIGN_M
	}

	fn opposite(self) -> bool {
		(self.u_a - self.far()).abs() < SIDE_ALIGN_M
	}

	fn adjacent(self) -> bool {
		self.v_a.abs() > self.hw - ADJACENT_RIM_M
			&& self.u_a > IN_WELL_SLACK_M
			&& self.u_a < self.far() - IN_WELL_SLACK_M
	}
}

fn plan_path(polyline: &FlightPolyline, fit: WellFit, width: f32, pref_depth: f32) -> Vec<Vec2> {
	let Some(frame) = Frame::new(fit) else {
		return plan_one_shot(polyline, fit);
	};
	if !frame.in_well() {
		return plan_one_shot(polyline, fit);
	}

	let rise = polyline.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let n_from_rise = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0);
	let needed = n_from_rise * pref_depth.max(1e-4);
	let far = frame.far();
	let i_len =
		if frame.same_side() { far } else { (xz(fit.upper_walk_on) - frame.start).length() };
	let l_len = frame.u_a.abs() + frame.v_a.abs();

	if needed <= i_len * COMPACT_FIT && (frame.opposite() || frame.same_side()) {
		return path_i(frame);
	}
	if frame.adjacent() && needed <= l_len * COMPACT_FIT {
		return path_l(frame);
	}

	let run = (far - width).max(width);
	path_switchback(frame, needed, run, width)
}

fn path_i(frame: Frame) -> Vec<Vec2> {
	if frame.same_side() {
		vec![frame.world(0.0, 0.0), frame.world(frame.far(), 0.0)]
	} else {
		vec![frame.world(0.0, 0.0), frame.world(frame.u_a, frame.v_a)]
	}
}

fn path_l(frame: Frame) -> Vec<Vec2> {
	dedup(vec![
		frame.world(0.0, 0.0),
		frame.world(frame.u_a, 0.0),
		frame.world(frame.u_a, frame.v_a),
	])
}

/// Two corridors, \(N\) out-and-backs stacked in \(Y\). Last L run stops at
/// \(u_a\); the walk-on is a landing, not a reverse on the last column.
fn path_switchback(frame: Frame, needed: f32, run: f32, width: f32) -> Vec<Vec2> {
	let far = frame.far();
	let (v0, v1) = corridor_pair(frame, width);
	let per_lap = (2.0 * run).max(1e-4);
	let last_leg = if frame.adjacent() {
		frame.u_a.clamp(width, far)
	} else if frame.opposite() {
		far
	} else {
		0.0
	};
	let rest = (needed - last_leg).max(0.0);
	let laps = (rest / per_lap).ceil() as u32;

	let mut local: Vec<(f32, f32)> = Vec::new();
	for _ in 0..laps {
		local.push((0.0, v0));
		local.push((far, v0));
		local.push((far, v1));
		local.push((0.0, v1));
	}
	if frame.adjacent() {
		local.push((0.0, v0));
		local.push((frame.u_a, v0));
	} else if frame.opposite() {
		local.push((0.0, v0));
		local.push((far, v0));
	}
	dedup(local.into_iter().map(|(u, v)| frame.world(u, v)).collect())
}

/// Opposite halves of the well: outbound away from the arrive, return toward it.
fn corridor_pair(frame: Frame, width: f32) -> (f32, f32) {
	let vmax = (frame.hw - 0.5 * width).max(0.0);
	let toward = if frame.v_a.abs() > SIDE_ALIGN_M { frame.v_a.signum() } else { 1.0 };
	let half = ((width + CORRIDOR_GAP_M) * 0.5).min(vmax);
	let v0 = (-toward * half).clamp(-vmax, vmax);
	let v1 = (toward * half).clamp(-vmax, vmax);
	(v0, v1)
}

fn plan_one_shot(polyline: &FlightPolyline, fit: WellFit) -> Vec<Vec2> {
	let start = xz(fit.lower_walk_on);
	let mut pts = vec![start];
	if polyline.stations.len() > 2 {
		for s in &polyline.stations[1..polyline.stations.len() - 1] {
			let p = xz(s.center);
			if pts.last().is_some_and(|q| (*q - p).length() > STATION_M) {
				pts.push(p);
			}
		}
	}
	let arrive = xz(fit.upper_walk_on);
	if pts.last().is_some_and(|q| (*q - arrive).length() > STATION_M) {
		pts.push(arrive);
	}
	if pts.len() < 2 || (pts.len() == 2 && (pts[1] - pts[0]).length() < STATION_M) {
		if let Some(out) = normalize_xz(fit.lower_out) {
			let span = (2.0 * fit.lower_half_depth.min(fit.upper_half_depth)).max(0.5);
			let far = start + out * span;
			if pts.len() < 2 {
				pts.push(far);
			} else {
				pts[1] = far;
			}
		}
	}
	pts
}

fn dedup(pts: Vec<Vec2>) -> Vec<Vec2> {
	let mut out: Vec<Vec2> = Vec::new();
	for p in pts {
		if out.last().is_some_and(|q| (*q - p).length() <= DEDUP_M) {
			if let Some(last) = out.last_mut() {
				*last = p;
			}
		} else {
			out.push(p);
		}
	}
	out
}

fn segs_along_path(path: &[Vec2], base_y: f32, rise: f32, width: f32) -> Vec<PathSeg> {
	let lens: Vec<f32> = path.windows(2).map(|w| (w[1] - w[0]).length()).collect();
	let run: Vec<f32> =
		lens.iter().map(|&len| if is_lateral(len, width) { 0.0 } else { len }).collect();
	let total: f32 = run.iter().sum();
	let mut y = base_y;
	let mut out = Vec::new();
	for (i, w) in path.windows(2).enumerate() {
		if (w[1] - w[0]).length() < EPS {
			continue;
		}
		let share = if total > EPS { rise * (run[i] / total) } else { 0.0 };
		out.push(PathSeg { start: w[0], end: w[1], y0: y, y1: y + share });
		y += share;
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stair_flights::geom::{run_top_y, travel_xz};
	use crate::stair_flights::{ComposedFlight, FlightPolyline, FlightStation, WellFit};
	use bevy_math::Vec3;
	use richmond_building_components::stairs::Stair;

	fn stacked_fit() -> (FlightPolyline, WellFit) {
		well(
			Vec3::new(0.0, 0.0, -1.2),
			Vec3::new(0.0, 3.0, -1.2),
			Vec2::Y,
			3.0,
			crate::stair_flights::geom::LAPPING_RATIO_DEFAULT,
		)
	}

	fn well(
		lower_walk: Vec3,
		upper_walk: Vec3,
		out: Vec2,
		rise: f32,
		lapping_ratio: f32,
	) -> (FlightPolyline, WellFit) {
		(
			FlightPolyline::new([
				FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: rise },
				FlightStation { center: Vec3::new(0.0, rise, 0.0), height: rise },
			]),
			WellFit {
				lower_center: Vec3::new(0.0, 0.0, 0.0),
				upper_center: Vec3::new(0.0, rise, 0.0),
				lower_walk_on: lower_walk,
				upper_walk_on: upper_walk,
				lower_out: out,
				lower_half_width: 1.2,
				lower_half_depth: 1.2,
				upper_half_width: 1.2,
				upper_half_depth: 1.2,
				tread_fill: crate::stair_flights::geom::TREAD_FILL_DEFAULT,
				lapping_ratio,
			},
		)
	}

	fn long_runs(flight: &ComposedFlight) -> Vec<&StairNode> {
		flight
			.stairs()
			.iter()
			.filter(|s| {
				let Stair::Straight(g) = &s.geometry;
				g.length > 0.45
			})
			.collect()
	}

	fn run_offset(a: &StairNode, right: Vec2) -> f32 {
		xz(a.placement.translation).dot(right)
	}

	#[test]
	fn stacked_is_a_side_by_side_u() {
		let (polyline, well) = stacked_fit();
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		let longs = long_runs(&flight);
		assert!(longs.len() >= 2, "aligned 180° should be a U, got {}", longs.len());
		assert!(!flight.pads().is_empty(), "far-rim rest must seat both treads");
		let right = Vec2::new(-1.0, 0.0);
		let out = Vec2::Y;
		let outs: Vec<_> = longs
			.iter()
			.filter(|s| travel_xz(s.placement.yaw).dot(out) > 0.5)
			.copied()
			.collect();
		let backs: Vec<_> = longs
			.iter()
			.filter(|s| travel_xz(s.placement.yaw).dot(out) < -0.5)
			.copied()
			.collect();
		assert!(!outs.is_empty() && !backs.is_empty(), "U needs an outbound and a return");
		let dv = (run_offset(outs[0], right) - run_offset(backs[0], right)).abs();
		assert!(dv > 0.3, "return must sit beside the outbound, offset={dv}");
		let return_z = backs[0].placement.translation.z;
		assert!(
			return_z < 0.85,
			"return must start at the landing inner edge, not side-on at the far rim, z={return_z}"
		);
		let ret = backs[0];
		let Stair::Straight(g) = &ret.geometry;
		let travel = travel_xz(ret.placement.yaw);
		let trail = xz(ret.placement.translation) - travel * (0.5 * g.going_per_tread());
		let pad = flight.pads().first().expect("U pad");
		for c in pad.corners() {
			let p = Vec2::new(c.x, c.z);
			let ahead = (p - trail).dot(travel);
			assert!(
				ahead <= 0.06,
				"first tread must start at the landing edge, not on it, ahead={ahead} p={p:?}"
			);
		}
		let inbound = travel_xz(outs[0].placement.yaw);
		let across = Vec2::new(-inbound.y, inbound.x);
		let mid0 = xz(outs[0].placement.translation);
		let mid1 = xz(ret.placement.translation);
		let sign = if (mid1 - mid0).dot(across) >= 0.0 { 1.0 } else { -1.0 };
		let outer = trail + across * sign * (0.5 * g.width);
		let reach = pad
			.corners()
			.into_iter()
			.map(|c| (Vec2::new(c.x, c.z) - outer).length())
			.fold(f32::MAX, f32::min);
		assert!(
			reach < 0.2,
			"landing must run the full second-flight width, outer={outer:?} nearest={reach}"
		);
		let last = flight.last_tread_xz();
		assert!(
			(last - Vec2::new(0.0, -1.2)).length() < 1.1,
			"last tread should arrive on the aligned walk-on, got {last:?}"
		);
	}

	#[test]
	fn short_rise_stays_one_crossing() {
		let (polyline, well) = well(
			Vec3::new(0.0, 0.0, -1.2),
			Vec3::new(0.0, 1.1, -1.2),
			Vec2::Y,
			1.1,
			crate::stair_flights::geom::LAPPING_RATIO_DEFAULT,
		);
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		assert_eq!(flight.stairs().len(), 1, "1.1 m < one 2.4 m crossing");
		assert!(flight.pads().is_empty());
	}

	#[test]
	fn quarter_turn_is_an_l_or_side_by_side() {
		let (polyline, well) = well(
			Vec3::new(0.0, 0.0, -1.2),
			Vec3::new(-1.2, 3.0, 0.0),
			Vec2::Y,
			3.0,
			crate::stair_flights::geom::LAPPING_RATIO_DEFAULT,
		);
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		assert!(
			flight.stairs().len() >= 2,
			"quarter-turn should turn, got {}",
			flight.stairs().len()
		);
		assert_arrives_west(&flight, "quarter-turn 0.55");
		assert_no_reverse_on_same_corridor(&flight, "quarter-turn 0.55");
		assert_runs_are_walkable(&flight, "quarter-turn 0.55");
		departing_treads_clear_pads(&flight, "quarter-turn 0.55");
	}

	fn assert_runs_are_walkable(flight: &ComposedFlight, label: &str) {
		for (i, s) in flight.stairs().iter().enumerate() {
			let Stair::Straight(g) = &s.geometry;
			assert!(
				g.width > 0.9,
				"{label}: run {i} width {} should fill half the 2.4 m well",
				g.width
			);
			assert!(
				g.going_per_tread() > 0.22,
				"{label}: run {i} going {} is a card deck",
				g.going_per_tread()
			);
		}
	}

	fn assert_arrives_west(flight: &ComposedFlight, label: &str) {
		let walk = Vec2::new(-1.2, 0.0);
		let last = flight.last_tread_xz();
		let last_d = (last - walk).length();
		if last_d < 0.85 {
			return;
		}
		let reach = flight
			.pads()
			.iter()
			.flat_map(|p| p.corners())
			.map(|c| (Vec2::new(c.x, c.z) - walk).length())
			.fold(f32::MAX, f32::min);
		assert!(
			reach < 0.25,
			"{label}: last tread {last:?} is {last_d:.2} from west; arrive pad nearest={reach}"
		);
	}

	fn assert_no_reverse_on_same_corridor(flight: &ComposedFlight, label: &str) {
		let right = Vec2::new(-1.0, 0.0);
		let longs = long_runs(flight);
		for pair in longs.windows(2) {
			let dv = (run_offset(pair[0], right) - run_offset(pair[1], right)).abs();
			if dv > 0.15 {
				continue;
			}
			let t0 = travel_xz(pair[0].placement.yaw);
			let t1 = travel_xz(pair[1].placement.yaw);
			assert!(t0.dot(t1) > 0.5, "{label}: consecutive runs reverse on the same corridor");
		}
	}

	fn departing_treads_clear_pads(flight: &ComposedFlight, label: &str) {
		for (i, pad) in flight.pads().iter().enumerate() {
			let Some(next) = flight.stairs().get(i + 1) else {
				continue;
			};
			let Stair::Straight(g) = &next.geometry;
			let travel = travel_xz(next.placement.yaw);
			let trail = xz(next.placement.translation) - travel * (0.5 * g.going_per_tread());
			let max_ahead = pad
				.corners()
				.into_iter()
				.map(|c| (Vec2::new(c.x, c.z) - trail).dot(travel))
				.fold(f32::NEG_INFINITY, f32::max);
			if max_ahead > g.going_per_tread() {
				continue;
			}
			assert!(
				max_ahead <= 0.06,
				"{label}: pad {i} is {max_ahead} along the next run; first trailing={trail:?} going={}",
				g.going_per_tread()
			);
		}
	}

	#[test]
	fn extra_lapping_reuses_two_corridors() {
		let (polyline, well) =
			well(Vec3::new(0.0, 0.0, -1.2), Vec3::new(-1.2, 3.0, 0.0), Vec2::Y, 3.0, 1.2);
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		let longs = long_runs(&flight);
		assert!(
			longs.len() >= 3,
			"lapping_ratio 1.2 should stack extra switchbacks, got {}",
			longs.len()
		);
		let right = Vec2::new(-1.0, 0.0);
		let out = Vec2::Y;
		let mut laterals: Vec<f32> = longs
			.iter()
			.filter(|s| travel_xz(s.placement.yaw).dot(out).abs() > 0.85)
			.map(|s| run_offset(s, right))
			.collect();
		laterals.sort_by(|a, b| a.partial_cmp(b).unwrap());
		laterals.dedup_by(|a, b| (*a - *b).abs() < 0.15);
		assert_eq!(
			laterals.len(),
			2,
			"extra lapping reuses two corridors, slots={}",
			laterals.len()
		);
		assert_arrives_west(&flight, "quarter-turn 1.2");
		assert_no_reverse_on_same_corridor(&flight, "quarter-turn 1.2");
		assert_runs_are_walkable(&flight, "quarter-turn 1.2");
		departing_treads_clear_pads(&flight, "quarter-turn 1.2");
	}

	#[test]
	fn runs_meet_pads_without_y_gaps() {
		let (polyline, well) =
			well(Vec3::new(0.0, 0.0, -1.2), Vec3::new(-1.2, 3.0, 0.0), Vec2::Y, 3.0, 1.2);
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		for pair in flight.stairs().windows(2) {
			let top = run_top_y(&pair[0]);
			let next = pair[1].placement.translation.y;
			assert!(
				(next - top).abs() < 1e-3,
				"next run must walk off the last tread, prev_top={top} next_y0={next}"
			);
		}
		let last_top = run_top_y(flight.stairs().last().expect("stairs"));
		assert!(
			(last_top - 3.0).abs() < 0.05,
			"flight should spend the full rise, last_top={last_top}"
		);
	}

	#[test]
	fn higher_lapping_ratio_adds_parallel_runs() {
		let (polyline, default_well) = stacked_fit();
		let default = fit(polyline.clone(), default_well, PanelStyle::RoughStonework, 0.05);
		let (polyline, deep_well) =
			well(Vec3::new(0.0, 0.0, -1.2), Vec3::new(0.0, 3.0, -1.2), Vec2::Y, 3.0, 2.0);
		let deep = fit(polyline, deep_well, PanelStyle::RoughStonework, 0.05);
		assert!(
			long_runs(&deep).len() > long_runs(&default).len(),
			"lapping_ratio 2.0 should add stacked switchbacks, {} vs {}",
			long_runs(&deep).len(),
			long_runs(&default).len()
		);
	}

	#[test]
	fn offset_polyline_gets_a_real_interior_landing() {
		let polyline = FlightPolyline::new([
			FlightStation { center: Vec3::new(0.0, 0.0, -3.0), height: 3.0 },
			FlightStation { center: Vec3::new(1.5, 1.5, -1.5), height: 3.0 },
			FlightStation { center: Vec3::new(3.0, 3.0, 0.0), height: 3.0 },
		]);
		let well = WellFit {
			lower_center: Vec3::new(0.0, 0.0, -3.0),
			upper_center: Vec3::new(3.0, 3.0, 0.0),
			lower_walk_on: Vec3::new(0.0, 0.0, -4.2),
			upper_walk_on: Vec3::new(3.0, 3.0, 1.2),
			lower_out: Vec2::Y,
			lower_half_width: 1.2,
			lower_half_depth: 1.2,
			upper_half_width: 1.2,
			upper_half_depth: 1.2,
			tread_fill: crate::stair_flights::geom::TREAD_FILL_DEFAULT,
			lapping_ratio: crate::stair_flights::geom::LAPPING_RATIO_DEFAULT,
		};
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		assert!(flight.stairs().len() >= 2, "offset should split into runs");
		let pad = flight.pads().first().expect("kink should author a pad");
		let [a, b, c, d] = pad.corners();
		let e0 = (b - a).length();
		let e1 = (c - a).length();
		assert!(e0 > 0.3 && e1 > 0.3, "interior landing should be a rectangle, edges={e0} {e1}");
		assert!(
			[a.y, b.y, c.y, d.y].iter().all(|y| (y - a.y).abs() < 0.05),
			"landing must stay level, ys={:?}",
			[a.y, b.y, c.y, d.y]
		);
	}
}
