//! Straight runs and landing pads at polyline kinks (L / U / offset).

use bevy_math::Vec2;
use richmond_building_components::panels::PanelStyle;

use crate::stair_flights::composed::ComposedFlight;
use crate::stair_flights::geom::{
	normalize_xz, place_runs_with_corner_landings, tread_dims, xz, PathSeg,
};
use crate::stair_flights::{FlightPolyline, WellFit};

/// Fit one straight run per polyline segment, with a rest at each kink.
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
) -> (
	Vec<richmond_building_components::stairs::StairNode>,
	Vec<crate::paneling::quad_panel::QuadPanel>,
) {
	let half_w = fit.lower_half_width.min(fit.upper_half_width).max(1e-4);
	let half_d = fit.lower_half_depth.min(fit.upper_half_depth).max(1e-4);
	let (width, pref_depth) = tread_dims(half_w.min(half_d));
	let waypoints = plan_waypoints(polyline, fit);
	let segs: Vec<PathSeg> = waypoints
		.windows(2)
		.map(|w| PathSeg { start: w[0].xz, end: w[1].xz, y0: w[0].y, y1: w[1].y })
		.collect();
	place_runs_with_corner_landings(&segs, width, pref_depth, style, thickness)
}

#[derive(Clone, Copy)]
struct Waypoint {
	xz: Vec2,
	y: f32,
}

fn plan_waypoints(polyline: &FlightPolyline, fit: WellFit) -> Vec<Waypoint> {
	let mut pts = vec![Waypoint { xz: xz(fit.lower_walk_on), y: fit.lower_walk_on.y }];
	if polyline.stations.len() > 2 {
		for s in &polyline.stations[1..polyline.stations.len() - 1] {
			let p = xz(s.center);
			if pts.last().map(|w| (w.xz - p).length() > 0.15).unwrap_or(true) {
				pts.push(Waypoint { xz: p, y: s.center.y });
			}
		}
	}
	let end = Waypoint { xz: xz(fit.upper_walk_on), y: fit.upper_walk_on.y };
	if pts.last().map(|w| (w.xz - end.xz).length() > 0.15).unwrap_or(true) {
		pts.push(end);
	} else if let Some(last) = pts.last_mut() {
		last.y = end.y;
	}

	if pts.len() < 2 || (pts.len() == 2 && (pts[1].xz - pts[0].xz).length() < 0.15) {
		let out = normalize_xz(fit.lower_out).unwrap_or(Vec2::X);
		let span = (2.0 * fit.lower_half_depth).max(0.5);
		let y1 = fit.upper_walk_on.y;
		if pts.len() < 2 {
			pts.push(Waypoint { xz: pts[0].xz + out * span, y: y1 });
		} else {
			pts[1].xz = pts[0].xz + out * span;
		}
	}
	pts
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stair_flights::{FlightPolyline, FlightStation, WellFit};
	use bevy_math::Vec3;

	fn stacked_fit() -> (FlightPolyline, WellFit) {
		(
			FlightPolyline::new([
				FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
				FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
			]),
			WellFit {
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
		)
	}

	#[test]
	fn stacked_is_one_run_without_kink_pad() {
		let (polyline, well) = stacked_fit();
		let flight = fit(polyline, well, PanelStyle::RoughStonework, 0.05);
		assert_eq!(flight.stairs().len(), 1);
		assert!(flight.pads().is_empty());
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
