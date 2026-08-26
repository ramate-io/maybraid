//! Circular flight: many one-tread [`StraightStair`] nodes around a centerline.

use std::f32::consts::TAU;

use bevy_math::{Vec2, Vec3};
use richmond_building_components::stairs::StraightStair;

use crate::stair_flights::composed::ComposedFlight;
use crate::stair_flights::geom::{normalize_xz, place_straight_run, travel_xz, xz};
use crate::stair_flights::{FlightPolyline, WellFit};
use richmond_building_components::stairs::StairNode;

/// Fit a circular run inside the shaft; outer rail on the lower walk-on.
pub fn fit(polyline: FlightPolyline, fit: WellFit) -> ComposedFlight {
	let stairs = fit_circular_nodes(&polyline, fit);
	ComposedFlight::new(polyline, stairs, Vec::new())
}

/// Place one-tread straight nodes on a circle. `start_yaw` sends local \(+X\)
/// toward the first tread (radial); kit travel is `start_yaw + θ + π/2`.
pub fn circular_straight_nodes(
	center: Vec3,
	start_yaw: f32,
	radius: f32,
	width: f32,
	depth: f32,
	local_tops: &[f32],
	turns: f32,
) -> Vec<StairNode> {
	if local_tops.is_empty() {
		return Vec::new();
	}
	let n = local_tops.len() as f32;
	let yaw_step = turns.max(1e-4) * TAU / n;
	let (ys, yc) = start_yaw.sin_cos();
	let rotate = |lx: f32, lz: f32| Vec2::new(yc * lx + ys * lz, -ys * lx + yc * lz);

	let mut prev_top = 0.0_f32;
	local_tops
		.iter()
		.copied()
		.enumerate()
		.filter_map(|(i, top)| {
			let rise = (top - prev_top).max(1e-4);
			prev_top = top;
			let theta = i as f32 * yaw_step;
			let (s, c) = theta.sin_cos();
			let p = rotate(c * radius, -s * radius);
			let travel_yaw = start_yaw + theta + std::f32::consts::FRAC_PI_2;
			place_straight_run(
				p + xz(center),
				center.y + top - rise,
				travel_xz(travel_yaw),
				width,
				depth,
				vec![rise],
			)
		})
		.collect()
}

fn fit_circular_nodes(polyline: &FlightPolyline, fit: WellFit) -> Vec<StairNode> {
	let rise = polyline.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let half_w = fit.lower_half_width.min(fit.upper_half_width).max(1e-4);
	let half_d = fit.lower_half_depth.min(fit.upper_half_depth).max(1e-4);
	let opening_min = half_w.min(half_d);
	let (tread_width, tread_depth) =
		crate::stair_flights::geom::tread_dims(opening_min, fit.tread_fill, fit.going_ratio);

	let (center, radius) = spiral_center_radius(polyline, fit, opening_min, tread_width);
	let turns = spiral_turns(rise, radius, tread_depth, fit, center);
	let yaw = spiral_start_yaw(fit.lower_walk_on, center, fit.lower_out);
	let n = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32;
	let local_tops = crate::stair_flights::geom::uniform_tops(rise, n);

	circular_straight_nodes(center, yaw, radius, tread_width, tread_depth, &local_tops, turns)
}

fn spiral_center_radius(
	polyline: &FlightPolyline,
	fit: WellFit,
	opening_min: f32,
	tread_width: f32,
) -> (Vec3, f32) {
	let a = xz(fit.lower_center);
	let b = xz(fit.upper_center);
	let mid = polyline.stations.get(1).map(|s| xz(s.center)).unwrap_or_else(|| (a + b) * 0.5);
	let center = Vec3::new(mid.x, fit.lower_center.y, mid.y);
	let half_tread = 0.5 * tread_width;
	let to_walk = (xz(fit.lower_walk_on) - mid).length();
	let inscribed = (opening_min - half_tread).max(0.35);
	let radius = (to_walk - half_tread).max(0.35).min(inscribed);
	(center, radius)
}

fn spiral_turns(rise: f32, radius: f32, tread_depth: f32, fit: WellFit, center: Vec3) -> f32 {
	let n = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0);
	let from_depth = n * tread_depth / (TAU * radius.max(1e-4));
	let from_arrive = arrive_turns(fit, center);
	from_depth.max(from_arrive).clamp(0.35, 3.0)
}

fn arrive_turns(fit: WellFit, center: Vec3) -> f32 {
	let c = xz(center);
	let from = xz(fit.lower_walk_on) - c;
	let to = xz(fit.upper_walk_on) - c;
	if from.length() < 1e-3 || to.length() < 1e-3 {
		return 0.35;
	}
	let a0 = from.y.atan2(from.x);
	let a1 = to.y.atan2(to.x);
	let mut delta = a1 - a0;
	while delta <= 0.0 {
		delta += TAU;
	}
	(delta / TAU).clamp(0.2, 1.5)
}

/// First tread is at local \(+X\); yaw sends that toward the lower walk-on.
fn spiral_start_yaw(walk_on: Vec3, center: Vec3, lower_out: Vec2) -> f32 {
	let mut toward = xz(walk_on) - xz(center);
	if toward.length() < 1e-3 {
		if let Some(out) = normalize_xz(lower_out) {
			toward = -out;
		} else {
			toward = Vec2::X;
		}
	} else {
		toward = toward.normalize();
	}
	(-toward.y).atan2(toward.x)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stair_flights::{FlightPolyline, FlightStation, WellFit};
	use lod::gen::LodSceneLevel;
	use richmond_building_components::BuildingComponents;

	#[test]
	fn fit_emits_straight_treads_inscribed_in_shaft() {
		let polyline = FlightPolyline::new([
			FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
			FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
		]);
		let flight = fit(
			polyline,
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
				tread_fill: crate::stair_flights::geom::TREAD_FILL_DEFAULT,
				going_ratio: crate::stair_flights::geom::GOING_RATIO_DEFAULT,
			},
		);
		assert!(!flight.stairs().is_empty());
		let first = flight.stairs()[0].placement.translation;
		let last = flight.last_tread_xz();
		assert!(
			(Vec2::new(first.x, first.z) - Vec2::new(0.0, -1.2)).length() < 0.6,
			"first tread should sit near the walk-on, got {first:?}"
		);
		assert!(last.length() > 0.3, "last tread should sit on the ring, got {last:?}");
		assert!(!flight.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}
}
