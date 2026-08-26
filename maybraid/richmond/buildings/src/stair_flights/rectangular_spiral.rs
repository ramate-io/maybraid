//! Rectangular-well spiral: a rim path; the shared placer owns runs and pads.

use bevy_math::Vec2;
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::StraightStair;

use crate::stair_flights::composed::ComposedFlight;
use crate::stair_flights::geom::{
	normalize_xz, place_runs_with_corner_landings, tread_dims, xz, PathSeg, EPS,
};
use crate::stair_flights::{FlightPolyline, WellFit};

/// Fit straight runs around the inscribed opening rim.
pub fn fit(
	polyline: FlightPolyline,
	fit: WellFit,
	style: PanelStyle,
	slab_thickness: f32,
) -> ComposedFlight {
	let (stairs, pads) = fit_rect_nodes(&polyline, fit, style, slab_thickness);
	ComposedFlight::new(polyline, stairs, pads)
}

fn fit_rect_nodes(
	polyline: &FlightPolyline,
	fit: WellFit,
	style: PanelStyle,
	thickness: f32,
) -> (
	Vec<richmond_building_components::stairs::StairNode>,
	Vec<crate::paneling::quad_panel::QuadPanel>,
) {
	let rise = polyline.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let half_w = fit.lower_half_width.min(fit.upper_half_width).max(1e-4);
	let half_d = fit.lower_half_depth.min(fit.upper_half_depth).max(1e-4);
	let (width, depth) = tread_dims(half_w.min(half_d));

	let center = xz(fit.lower_center);
	let out = normalize_xz(fit.lower_out).unwrap_or(Vec2::Y);
	let right = Vec2::new(-out.y, out.x);
	let inset = 0.5 * width;
	let hw = (half_w - inset).max(0.2);
	let hd = (half_d - inset).max(0.2);
	let corners = [
		center - out * hd - right * hw,
		center - out * hd + right * hw,
		center + out * hd + right * hw,
		center + out * hd - right * hw,
	];

	let start = (corners[0] + corners[1]) * 0.5;
	let mut loop_pts = vec![start];
	loop_pts.extend_from_slice(&corners[1..]);
	loop_pts.push(corners[0]);
	loop_pts.push(start);

	let lap_len = loop_length(&loop_pts);
	if lap_len < EPS {
		return (Vec::new(), Vec::new());
	}

	let n_from_rise = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0);
	let from_depth = n_from_rise * depth / lap_len;
	let from_arrive = arrive_laps(fit, &loop_pts, start);
	let extra = from_depth.floor().max(0.0) as u32;
	let frac = if from_arrive < 0.08 { 1.0 } else { from_arrive };

	let mut path = Vec::new();
	for _ in 0..extra {
		append_loop(&mut path, &loop_pts);
	}
	append_loop_frac(&mut path, &loop_pts, frac);
	if path.len() < 2 {
		append_loop(&mut path, &loop_pts);
	}

	let segs = segs_along_path(&path, fit.lower_center.y, rise);
	place_runs_with_corner_landings(&segs, width, depth, style, thickness)
}

fn segs_along_path(path: &[Vec2], base_y: f32, rise: f32) -> Vec<PathSeg> {
	let lens: Vec<f32> = path.windows(2).map(|w| (w[1] - w[0]).length()).collect();
	let total: f32 = lens.iter().sum();
	let mut y = base_y;
	let mut out = Vec::new();
	for (i, w) in path.windows(2).enumerate() {
		let share = if total > EPS { rise * (lens[i] / total) } else { 0.0 };
		out.push(PathSeg { start: w[0], end: w[1], y0: y, y1: y + share });
		y += share;
	}
	out
}

fn loop_length(pts: &[Vec2]) -> f32 {
	pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

fn append_loop(path: &mut Vec<Vec2>, loop_pts: &[Vec2]) {
	let skip_first = !path.is_empty();
	for (i, p) in loop_pts.iter().enumerate() {
		if skip_first && i == 0 {
			continue;
		}
		path.push(*p);
	}
}

fn append_loop_frac(path: &mut Vec<Vec2>, loop_pts: &[Vec2], frac: f32) {
	let total = loop_length(loop_pts);
	let target = (frac * total).max(0.0);
	let mut acc = 0.0;
	for w in loop_pts.windows(2) {
		let seg = (w[1] - w[0]).length();
		if acc + seg >= target && seg > EPS {
			let t = ((target - acc) / seg).clamp(0.0, 1.0);
			if path.last().map(|p| (*p - w[0]).length() > EPS).unwrap_or(true) {
				path.push(w[0]);
			}
			path.push(w[0] + (w[1] - w[0]) * t);
			return;
		}
		if path.last().map(|p| (*p - w[0]).length() > EPS).unwrap_or(true) {
			path.push(w[0]);
		}
		acc += seg;
	}
	if let Some(&last) = loop_pts.last() {
		path.push(last);
	}
}

fn arrive_laps(fit: WellFit, loop_pts: &[Vec2], start: Vec2) -> f32 {
	let target = nearest_on_loop(xz(fit.upper_walk_on), loop_pts).unwrap_or(start);
	let total = loop_length(loop_pts);
	if total < EPS {
		return 0.25;
	}
	let mut acc = 0.0;
	let mut best_d = f32::MAX;
	let mut best_s = 0.0;
	for w in loop_pts.windows(2) {
		let v = w[1] - w[0];
		let len = v.length();
		if len < EPS {
			continue;
		}
		let u = ((target - w[0]).dot(v) / (len * len)).clamp(0.0, 1.0);
		let q = w[0] + v * u;
		let d = (target - q).length();
		if d < best_d {
			best_d = d;
			best_s = acc + u * len;
		}
		acc += len;
	}
	let frac = best_s / total;
	if frac < 0.12 {
		1.0
	} else {
		frac.clamp(0.15, 1.0)
	}
}

fn nearest_on_loop(p: Vec2, loop_pts: &[Vec2]) -> Option<Vec2> {
	let mut best: Option<(f32, Vec2)> = None;
	for w in loop_pts.windows(2) {
		let v = w[1] - w[0];
		let len2 = v.length_squared();
		if len2 < EPS * EPS {
			continue;
		}
		let u = ((p - w[0]).dot(v) / len2).clamp(0.0, 1.0);
		let q = w[0] + v * u;
		let d2 = (p - q).length_squared();
		if best.map(|(bd, _)| d2 < bd).unwrap_or(true) {
			best = Some((d2, q));
		}
	}
	best.map(|(_, q)| q)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stair_flights::geom::{run_top_y, travel_xz, xz};
	use crate::stair_flights::{ComposedFlight, FlightPolyline, FlightStation, TreadEnd, WellFit};
	use bevy_math::{Vec2, Vec3};
	use lod::gen::LodSceneLevel;
	use richmond_building_components::partitions::PANEL_Y_HALF;
	use richmond_building_components::stairs::Stair;
	use richmond_building_components::BuildingComponents;

	#[test]
	fn stacked_well_emits_runs_and_corner_pads() {
		let flight = fit_rect_well(Vec3::new(0.0, 3.0, -1.2), 1.2);
		assert!(
			flight.stairs().len() >= 2,
			"rectangular spiral should turn at least once, got {}",
			flight.stairs().len()
		);
		assert!(!flight.pads().is_empty(), "midway corners should get rest pads");
		let [a, b, c, ..] = flight.pads()[0].corners();
		assert!(
			(b - a).length() > 0.3 && (c - a).length() > 0.3,
			"corner pad should be a rectangle"
		);
		assert!(!flight.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn joint_invariants_on_stacked_quarter_turn_and_tiny() {
		for (label, upper_walk, half) in [
			("quarter-turn", Vec3::new(1.2, 3.0, 0.0), 1.2),
			("stacked", Vec3::new(0.0, 3.0, -1.2), 1.2),
			("tiny", Vec3::new(0.0, 3.0, -0.45), 0.45),
		] {
			assert_joint_invariants(label, &fit_rect_well(upper_walk, half));
		}
	}

	fn assert_joint_invariants(label: &str, flight: &ComposedFlight) {
		let stairs = flight.stairs();
		let pads = flight.pads();
		assert!(
			stairs.len() >= 2 && !pads.is_empty() && stairs.len() >= pads.len() + 1,
			"{label}: need a pad per interior joint, stairs={} pads={}",
			stairs.len(),
			pads.len()
		);
		for pair in stairs.windows(2) {
			let top = run_top_y(&pair[0]);
			let next_y = pair[1].placement.translation.y;
			assert!(
				(next_y - top).abs() < 1e-3,
				"{label}: next.y0 should equal prev.y1, prev_top={top} next_y={next_y}"
			);
		}
		for (i, pad) in pads.iter().enumerate() {
			let last = TreadEnd::from_straight(&stairs[i]);
			let going = match &stairs[i + 1].geometry {
				Stair::Straight(g) => g.going_per_tread(),
			};
			let travel = travel_xz(stairs[i + 1].placement.yaw);
			let first = xz(stairs[i + 1].placement.translation);
			let trailing = first - travel * 0.5 * going;
			let incoming_top = run_top_y(&stairs[i]);
			for p in pad.corners() {
				assert!(
					(p.y - (incoming_top - PANEL_Y_HALF)).abs() < 1e-3,
					"{label}: pad {i} Y should be last-tread top minus PANEL_Y_HALF, got {} want {}",
					p.y,
					incoming_top - PANEL_Y_HALF
				);
			}
			for (name, p) in [("outer", last.leading_outer), ("inner", last.leading_inner)] {
				let (s, t) = pad_st(pad, p).unwrap_or((f32::NAN, f32::NAN));
				assert!(
					on_pad(s, t) && near_edge(s, t),
					"{label}: incoming leading {name} {p:?} not on pad near edge st=({s},{t}) corners={:?}",
					pad.corners()
				);
			}
			let (s, t) = pad_st(pad, trailing).unwrap_or((f32::NAN, f32::NAN));
			assert!(
				on_pad(s, t) && far_edge(s, t),
				"{label}: next first trailing {trailing:?} should sit on pad {i} far edge st=({s},{t})"
			);
		}
	}

	fn on_pad(s: f32, t: f32) -> bool {
		(-0.08..=1.08).contains(&s) && (-0.08..=1.08).contains(&t)
	}

	fn near_edge(s: f32, t: f32) -> bool {
		s.abs() < 0.12 || t.abs() < 0.12
	}

	fn far_edge(s: f32, t: f32) -> bool {
		(s - 1.0).abs() < 0.12 || (t - 1.0).abs() < 0.12
	}

	fn fit_rect_well(upper_walk_on: Vec3, half: f32) -> ComposedFlight {
		fit(
			FlightPolyline::new([
				FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
				FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
			]),
			WellFit {
				lower_center: Vec3::new(0.0, 0.0, 0.0),
				upper_center: Vec3::new(0.0, 3.0, 0.0),
				lower_walk_on: Vec3::new(0.0, 0.0, -half),
				upper_walk_on,
				lower_out: Vec2::Y,
				lower_half_width: half,
				lower_half_depth: half,
				upper_half_width: half,
				upper_half_depth: half,
			},
			PanelStyle::RoughStonework,
			0.05,
		)
	}

	fn pad_st(pad: &crate::paneling::quad_panel::QuadPanel, p: Vec2) -> Option<(f32, f32)> {
		let [a0, a1, b0, ..] = pad.corners();
		let a0 = Vec2::new(a0.x, a0.z);
		let u = Vec2::new(a1.x, a1.z) - a0;
		let v = Vec2::new(b0.x, b0.z) - a0;
		let w = p - a0;
		let uu = u.dot(u);
		let vv = v.dot(v);
		let uv = u.dot(v);
		let denom = uu * vv - uv * uv;
		if denom.abs() < 1e-8 {
			return None;
		}
		let s = (w.dot(u) * vv - w.dot(v) * uv) / denom;
		let t = (w.dot(v) * uu - w.dot(u) * uv) / denom;
		Some((s, t))
	}
}
