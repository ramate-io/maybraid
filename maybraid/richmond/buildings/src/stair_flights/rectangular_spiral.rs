//! Rectangular-well spiral: straight runs around an inset eye, landings at corners.

use bevy_math::Vec2;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::{StairNode, StraightStair};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::geom::{
	normalize_xz, place_runs_with_corner_landings, tread_dims, xz, PathSeg, EPS,
};
use crate::stair_flights::{FlightPolyline, SpiralFlightFit};

/// Rectangular spiral: straight runs around a rectangular eye, rest pads at corners.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularSpiralFlight {
	polyline: FlightPolyline,
	stairs: Vec<StairNode>,
	corner_pads: Vec<QuadPanel>,
}

impl RectangularSpiralFlight {
	pub fn new(polyline: FlightPolyline, stairs: Vec<StairNode>, corner_pads: Vec<QuadPanel>) -> Self {
		Self { polyline, stairs, corner_pads }
	}

	/// Fit straight runs around the inscribed opening rim.
	pub fn fit(
		polyline: FlightPolyline,
		fit: SpiralFlightFit,
		style: PanelStyle,
		slab_thickness: f32,
	) -> Self {
		let (stairs, corner_pads) = fit_rect_nodes(&polyline, fit, style, slab_thickness);
		Self { polyline, stairs, corner_pads }
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}

	pub fn stairs(&self) -> &[StairNode] {
		&self.stairs
	}

	pub fn corner_pads(&self) -> &[QuadPanel] {
		&self.corner_pads
	}

	pub fn last_tread_xz(&self) -> Vec2 {
		crate::stair_flights::TreadEnd::from_last_straight(&self.stairs)
			.map(|e| e.leading_mid() - e.travel * 0.01)
			.or_else(|| self.stairs.last().map(|n| xz(n.placement.translation)))
			.unwrap_or(Vec2::ZERO)
	}

	pub fn last_tread_travel_xz(&self) -> Vec2 {
		crate::stair_flights::TreadEnd::from_last_straight(&self.stairs)
			.map(|e| e.travel)
			.unwrap_or(Vec2::X)
	}

	pub fn last_tread_leading_xz(&self) -> (Vec2, Vec2) {
		crate::stair_flights::TreadEnd::from_last_straight(&self.stairs)
			.map(|e| (e.leading_outer, e.leading_inner))
			.unwrap_or_else(|| {
				let p = self.last_tread_xz();
				(p, p)
			})
	}
}

impl BuildingComponents for RectangularSpiralFlight {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::from_free(self.stairs.clone())
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for pad in &self.corner_pads {
			out.extend(pad.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for pad in &self.corner_pads {
			out.extend(pad.joint_nodes_for_level(level));
		}
		out
	}
}

fn fit_rect_nodes(
	polyline: &FlightPolyline,
	fit: SpiralFlightFit,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Vec<QuadPanel>) {
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
	// Extra full laps for rise, then always finish on the arrive fraction so
	// the last run ends at the upper walk-on — not mid-side with a sheared pad.
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

fn arrive_laps(fit: SpiralFlightFit, loop_pts: &[Vec2], start: Vec2) -> f32 {
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
	// Same-side arrive is at the start: walk a full lap, don't stop 15% in.
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
	use crate::stair_flights::geom::xz;
	use crate::stair_flights::{FlightPolyline, FlightStation, TreadEnd};
	use bevy_math::{Vec2, Vec3};

	#[test]
	fn stacked_well_emits_runs_and_corner_pads() {
		let polyline = FlightPolyline::new([
			FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
			FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
		]);
		let flight = RectangularSpiralFlight::fit(
			polyline,
			SpiralFlightFit {
				lower_center: Vec3::new(0.0, 0.0, 0.0),
				upper_center: Vec3::new(0.0, 3.0, 0.0),
				lower_walk_on: Vec3::new(0.0, 0.0, -1.2),
				upper_walk_on: Vec3::new(0.0, 3.0, -1.2),
				lower_out: Vec2::Y,
				lower_half_width: 1.2,
				lower_half_depth: 1.2,
				upper_half_width: 1.2,
				upper_half_depth: 1.2,
			},
			PanelStyle::RoughStonework,
			0.05,
		);
		assert!(
			flight.stairs().len() >= 2,
			"rectangular spiral should turn at least once, got {}",
			flight.stairs().len()
		);
		assert!(!flight.corner_pads().is_empty(), "midway corners should get rest pads");
		let [a, b, c, ..] = flight.corner_pads()[0].corners();
		assert!((b - a).length() > 0.3 && (c - a).length() > 0.3, "corner pad should be a rectangle");
		assert!(!flight.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn corner_pads_fill_width_and_do_not_swallow_treads() {
		for (label, upper_walk, half) in [
			("quarter-turn", Vec3::new(1.2, 3.0, 0.0), 1.2),
			("stacked", Vec3::new(0.0, 3.0, -1.2), 1.2),
			("tiny", Vec3::new(0.0, 3.0, -0.45), 0.45),
		] {
			let flight = fit_rect_well(upper_walk, half);
			assert!(
				flight.stairs().len() >= 2 && !flight.corner_pads().is_empty(),
				"{label}: need a corner pad and a following run"
			);
			for (i, pad) in flight.corner_pads().iter().enumerate() {
				let Some(incoming) = flight.stairs().get(i) else {
					continue;
				};
				let Some(next) = flight.stairs().get(i + 1) else {
					continue;
				};
				let [a0, a1, b0, ..] = pad.corners();
				let last = TreadEnd::from_straight(incoming);
				let pad_mid = Vec2::new(
					(a0.x + a1.x + b0.x) / 3.0,
					(a0.z + a1.z + b0.z) / 3.0,
				);
				if (last.leading_mid() - pad_mid).length() > 0.7 {
					continue;
				}
				let first = xz(next.placement.translation);
				if half >= 0.6 {
					assert!(
						!point_in_pad_xz(a0, a1, b0, first, 0.12),
						"{label}: next run starts on the pad, first={first:?}"
					);
				}
				let last_mid = last.leading_mid() - last.travel * 0.01;
				assert!(
					!point_in_pad_xz(a0, a1, b0, last_mid, 0.08),
					"{label}: pad swallows the last incoming tread, mid={last_mid:?}"
				);
				for (name, p) in [("outer", last.leading_outer), ("inner", last.leading_inner)] {
					assert!(
						point_in_pad_xz(a0, a1, b0, p, -0.06),
						"{label}: pad misses incoming leading {name} {p:?} pad={:?}",
						pad.corners()
					);
				}
			}
		}
	}

	#[test]
	fn tiny_well_keeps_runs_connected() {
		let flight = fit_rect_well(Vec3::new(0.0, 3.0, -0.45), 0.45);
		let stairs = flight.stairs();
		assert!(stairs.len() >= 3, "tiny well should still walk more than one side, got {}", stairs.len());
		for pair in stairs.windows(2) {
			let top = match &pair[0].geometry {
				richmond_building_components::stairs::Stair::Straight(g) => {
					pair[0].placement.translation.y + g.height
				}
			};
			let next_y = pair[1].placement.translation.y;
			assert!(
				(next_y - top).abs() < 0.08,
				"tiny: Y gap between runs, prev_top={top} next_y={next_y}"
			);
			let a = crate::stair_flights::TreadEnd::from_straight(&pair[0]).leading_mid();
			let b = xz(pair[1].placement.translation);
			assert!(
				(a - b).length() < 0.85,
				"tiny: plan gap between runs, last={a:?} first={b:?}"
			);
		}
	}

	fn fit_rect_well(upper_walk_on: Vec3, half: f32) -> RectangularSpiralFlight {
		RectangularSpiralFlight::fit(
			FlightPolyline::new([
				FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
				FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
			]),
			SpiralFlightFit {
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

	fn point_in_pad_xz(a0: Vec3, a1: Vec3, b0: Vec3, p: Vec2, inset: f32) -> bool {
		let a0 = Vec2::new(a0.x, a0.z);
		let u = Vec2::new(a1.x, a1.z) - a0;
		let v = Vec2::new(b0.x, b0.z) - a0;
		let w = p - a0;
		let uu = u.dot(u);
		let vv = v.dot(v);
		let uv = u.dot(v);
		let denom = uu * vv - uv * uv;
		if denom.abs() < 1e-8 {
			return false;
		}
		let s = (w.dot(u) * vv - w.dot(v) * uv) / denom;
		let t = (w.dot(v) * uu - w.dot(u) * uv) / denom;
		s > inset && s < 1.0 - inset && t > inset && t < 1.0 - inset
	}
}
