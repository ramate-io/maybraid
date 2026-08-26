//! I / L / U flights: openings own the ends; extra lapping is a side-by-side bank.
//!
//! Walk-on sides pick the graph. Adjacent → compact L. Same side → U (two
//! parallel runs, far landing spans both). Opposite or a short rise → I.
//! Lapping ratio only packs that graph: if one topology is short, add
//! another **lateral** run (never the same centerline twice). A long L
//! between offset openings stays the polyline path.

use bevy_math::Vec2;
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::{StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::composed::ComposedFlight;
use crate::stair_flights::geom::{
	is_lateral, normalize_xz, place_runs_with_corner_landings, xz, PathSeg, EPS,
};
use crate::stair_flights::{FlightPolyline, WellFit};

/// Plan points closer than this are the same waypoint.
const DEDUP_M: f32 = 0.12;
/// Walk-on is on this side of the well if |u| or |u − far| is below this.
const SIDE_ALIGN_M: f32 = 0.35;
const IN_WELL_SLACK_M: f32 = 0.3;
const ADJACENT_RIM_M: f32 = 0.4;
/// Compact I / L is enough when needed going fits this × the path length.
const COMPACT_FIT: f32 = 1.05;
const STATION_M: f32 = 0.15;

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
	let (width, pref_depth) = fit.tread_dims();
	let rise = polyline.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let path = plan_path(polyline, fit, width, pref_depth);
	let segs = segs_along_path(&path, fit.lower_walk_on.y, rise, width);
	place_runs_with_corner_landings(&segs, width, pref_depth, style, thickness)
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
	path_bank(frame, bank_slots(frame, needed, run, width), width)
}

/// Side-by-side slots so rise fits `run`, even parity on a U, odd on an I.
fn bank_slots(frame: Frame, needed: f32, run: f32, width: f32) -> u32 {
	let max_slots = ((2.0 * frame.hw) / width.max(1e-4)).floor().max(1.0) as u32;
	let mut n = (needed / run).ceil().max(1.0) as u32;
	if frame.same_side() {
		n = n.max(2);
		if n % 2 == 1 {
			if n > 2 && (n - 1) as f32 * run >= needed * 0.85 {
				n -= 1;
			} else {
				n += 1;
			}
		}
	} else if frame.opposite() && n % 2 == 0 {
		n += 1;
	}
	n.min(max_slots).max(1)
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

fn path_bank(frame: Frame, n: u32, width: f32) -> Vec<Vec2> {
	let far = frame.far();
	let prefer = if frame.adjacent() { frame.v_a } else { 0.0 };
	let slots = slot_vs(n, width, frame.hw, prefer);
	let mut local: Vec<(f32, f32)> = Vec::new();
	for (k, &v) in slots.iter().enumerate() {
		if k % 2 == 0 {
			local.push((0.0, v));
			local.push((far, v));
		} else {
			local.push((far, v));
			local.push((0.0, v));
		}
	}
	if frame.adjacent() {
		if let Some(&(u, v)) = local.last() {
			if (u - frame.u_a).abs() > DEDUP_M {
				local.push((frame.u_a, v));
			}
			if (v - frame.v_a).abs() > DEDUP_M {
				local.push((frame.u_a, frame.v_a));
			}
		}
	} else if frame.opposite() {
		if let Some(&(_, v)) = local.last() {
			if (v - frame.v_a).abs() > DEDUP_M {
				local.push((far, frame.v_a));
			}
		}
	}
	dedup(local.into_iter().map(|(u, v)| frame.world(u, v)).collect())
}

fn slot_vs(n: u32, width: f32, hw: f32, prefer_last: f32) -> Vec<f32> {
	let n = n.max(1);
	let span = (n - 1) as f32 * width;
	let vmax = (hw - 0.5 * width).max(0.0);
	let vmin = -vmax;
	let v_last = prefer_last.clamp(vmin, vmax);
	let v0 = (v_last - span).clamp(vmin, vmax.min((vmax - span).max(vmin)));
	(0..n).map(|k| v0 + k as f32 * width).collect()
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
				g.length > 0.8
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
		let dv = (run_offset(longs[0], right) - run_offset(longs[1], right)).abs();
		assert!(dv > 0.3, "return must sit beside the outbound, offset={dv}");
		let return_z = longs[1].placement.translation.z;
		assert!(
			return_z < 0.85,
			"return must start at the landing inner edge, not side-on at the far rim, z={return_z}"
		);
		let ret = longs[1];
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
		let inbound = travel_xz(longs[0].placement.yaw);
		let across = Vec2::new(-inbound.y, inbound.x);
		let mid0 = xz(longs[0].placement.translation);
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
		let last = flight.last_tread_xz();
		assert!(
			(last - Vec2::new(-1.2, 0.0)).length() < 0.75,
			"last tread should meet the west walk-on, got {last:?}"
		);
		departing_treads_clear_pads(&flight, "quarter-turn 0.55");
	}

	fn departing_treads_clear_pads(flight: &ComposedFlight, label: &str) {
		for (i, pad) in flight.pads().iter().enumerate() {
			let Some(next) = flight.stairs().get(i + 1) else {
				continue;
			};
			let Stair::Straight(g) = &next.geometry;
			let travel = travel_xz(next.placement.yaw);
			let trail = xz(next.placement.translation) - travel * (0.5 * g.going_per_tread());
			for c in pad.corners() {
				let p = Vec2::new(c.x, c.z);
				let ahead = (p - trail).dot(travel);
				assert!(
					ahead <= 0.06,
					"{label}: pad {i} corner {p:?} is {ahead} along the next run; first trailing={trail:?} going={}",
					g.going_per_tread()
				);
			}
		}
	}

	#[test]
	fn extra_lapping_uses_side_by_side_runs() {
		let (polyline, well) =
			well(Vec3::new(0.0, 0.0, -1.2), Vec3::new(-1.2, 3.0, 0.0), Vec2::Y, 3.0, 1.2);
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		let longs = long_runs(&flight);
		assert!(
			longs.len() >= 3,
			"lapping_ratio 1.2 should pack extra parallel runs, got {}",
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
		assert!(
			laterals.len() >= 2,
			"lapping_ratio 1.2 should occupy more than one corridor, slots={}",
			laterals.len()
		);
		let last = flight.last_tread_xz();
		assert!(
			(last - Vec2::new(-1.2, 0.0)).length() < 0.85,
			"still arrive on the west walk-on, got {last:?}"
		);
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
			"lapping_ratio 2.0 should add lateral runs, {} vs {}",
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
