//! Circular / spiral run along a [`super::FlightPolyline`].

use std::f32::consts::TAU;

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::stairs::{SpiralStair, Stair, StairNode};
use richmond_building_components::{BuildingComponents, Layers, Placement};

use crate::stair_flights::FlightPolyline;

/// Inputs for fitting a spiral inside a vertical shaft (two horizontal faces).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralFlightFit {
	pub lower_center: Vec3,
	pub upper_center: Vec3,
	pub lower_walk_on: Vec3,
	pub upper_walk_on: Vec3,
	/// XZ walk-off from the lower walk-on into the well.
	pub lower_out: bevy_math::Vec2,
	pub lower_half_width: f32,
	pub lower_half_depth: f32,
	pub upper_half_width: f32,
	pub upper_half_depth: f32,
}

/// Spiral flight composed from existing [`StairNode`] IR.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralFlight {
	polyline: FlightPolyline,
	stairs: StairNode,
}

impl SpiralFlight {
	pub fn new(polyline: FlightPolyline, stairs: StairNode) -> Self {
		Self { polyline, stairs }
	}

	/// Fit a circular run inside the shaft; outer rail on the lower walk-on.
	pub fn fit(polyline: FlightPolyline, fit: SpiralFlightFit) -> Self {
		let stairs = fit_spiral_node(&polyline, fit);
		Self { polyline, stairs }
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}

	pub fn stairs(&self) -> &StairNode {
		&self.stairs
	}
}

impl BuildingComponents for SpiralFlight {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::from_free(vec![self.stairs.clone()])
	}
}

fn fit_spiral_node(polyline: &FlightPolyline, fit: SpiralFlightFit) -> StairNode {
	let rise = polyline.rise().max(SpiralStair::DEFAULT_TREAD_HEIGHT);
	let half_w = fit.lower_half_width.min(fit.upper_half_width).max(1e-4);
	let half_d = fit.lower_half_depth.min(fit.upper_half_depth).max(1e-4);
	let opening_min = half_w.min(half_d);
	let tread_width = (opening_min * 0.4).clamp(0.35, 1.1);
	let tread_depth = (tread_width * 0.55).clamp(0.25, 0.45);

	let (center, radius) = spiral_center_radius(polyline, fit, opening_min, tread_width);
	let turns = spiral_turns(rise, radius, tread_depth, fit, center);
	let yaw = spiral_start_yaw(fit.lower_walk_on, center, fit.lower_out);

	let geometry = Stair::Spiral(SpiralStair {
		height: rise,
		radius,
		tread_width,
		tread_depth,
		tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
		turns,
		tread_tops: Vec::new(),
	});
	StairNode::rough_stone(geometry, Placement::new(center, yaw))
}

fn spiral_center_radius(
	polyline: &FlightPolyline,
	fit: SpiralFlightFit,
	opening_min: f32,
	tread_width: f32,
) -> (Vec3, f32) {
	let a = xz(fit.lower_center);
	let b = xz(fit.upper_center);
	let mid = polyline.stations.get(1).map(|s| xz(s.center)).unwrap_or_else(|| (a + b) * 0.5);
	let center = Vec3::new(mid.x, fit.lower_center.y, mid.y);
	// `radius` is the tread centerline. Inset by half tread width so the outer
	// rail sits on the walk-on / hole edge; the run-in covers the remaining gap.
	let half_tread = 0.5 * tread_width;
	let to_walk = (xz(fit.lower_walk_on) - mid).length();
	let inscribed = (opening_min - half_tread).max(0.35);
	let radius = (to_walk - half_tread).max(0.35).min(inscribed);
	(center, radius)
}

fn spiral_turns(
	rise: f32,
	radius: f32,
	tread_depth: f32,
	fit: SpiralFlightFit,
	center: Vec3,
) -> f32 {
	let n = (rise / SpiralStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0);
	let from_depth = n * tread_depth / (TAU * radius.max(1e-4));
	let from_arrive = arrive_turns(fit, center);
	from_depth.max(from_arrive).clamp(0.35, 3.0)
}

fn arrive_turns(fit: SpiralFlightFit, center: Vec3) -> f32 {
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

/// First spiral tread is at local \(+X\); yaw sends that toward the lower walk-on.
fn spiral_start_yaw(walk_on: Vec3, center: Vec3, lower_out: bevy_math::Vec2) -> f32 {
	let mut toward = xz(walk_on) - xz(center);
	if toward.length() < 1e-3 {
		if let Some(out) = normalize_xz(lower_out) {
			toward = -out;
		} else {
			toward = bevy_math::Vec2::X;
		}
	} else {
		toward = toward.normalize();
	}
	// Placement yaw: local +X → `(cos yaw, 0, -sin yaw)` = toward walk-on.
	(-toward.y).atan2(toward.x)
}

fn xz(p: Vec3) -> bevy_math::Vec2 {
	bevy_math::Vec2::new(p.x, p.z)
}

fn normalize_xz(v: bevy_math::Vec2) -> Option<bevy_math::Vec2> {
	let len = v.length();
	if len < 1e-5 {
		None
	} else {
		Some(v / len)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stair_flights::{FlightPolyline, FlightStation};

	#[test]
	fn fit_emits_spiral_inscribed_in_shaft() {
		let polyline = FlightPolyline::new([
			FlightStation { center: Vec3::new(0.0, 0.0, 0.0), height: 3.0 },
			FlightStation { center: Vec3::new(0.0, 3.0, 0.0), height: 3.0 },
		]);
		let flight = SpiralFlight::fit(
			polyline,
			SpiralFlightFit {
				lower_center: Vec3::new(0.0, 0.0, 0.0),
				upper_center: Vec3::new(0.0, 3.0, 0.0),
				lower_walk_on: Vec3::new(0.0, 0.0, -1.2),
				upper_walk_on: Vec3::new(0.0, 3.0, -1.2),
				lower_out: bevy_math::Vec2::Y,
				lower_half_width: 1.2,
				lower_half_depth: 1.2,
				upper_half_width: 1.2,
				upper_half_depth: 1.2,
			},
		);
		let Stair::Spiral(g) = &flight.stairs().geometry else {
			panic!("expected spiral");
		};
		assert!(g.height > 2.9);
		assert!((g.radius + 0.5 * g.tread_width - 1.2).abs() < 1e-3, "outer rail should sit on the hole, radius={}", g.radius);
		let c = flight.stairs().placement.translation;
		assert!(c.x.abs() < 0.15 && c.z.abs() < 0.15, "center={c:?}");
		let first = first_tread_xz(&flight);
		let outer = first + (first - bevy_math::Vec2::new(c.x, c.z)).normalize() * (0.5 * g.tread_width);
		assert!(
			(outer - bevy_math::Vec2::new(0.0, -1.2)).length() < 0.05,
			"outer rail should sit on the walk-on, centerline={first:?} outer={outer:?}"
		);
		assert!(!flight.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	fn first_tread_xz(flight: &SpiralFlight) -> bevy_math::Vec2 {
		let Stair::Spiral(g) = &flight.stairs().geometry else {
			panic!("expected spiral");
		};
		let p = flight.stairs().placement;
		// Local first tread at +X * radius; yaw maps +X → (cos, 0, -sin).
		let (s, c) = p.yaw.sin_cos();
		bevy_math::Vec2::new(p.translation.x + c * g.radius, p.translation.z + -s * g.radius)
	}
}
