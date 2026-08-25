//! Circular / spiral run along a [`super::FlightPolyline`].

use std::f32::consts::TAU;

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::stairs::{SpiralStair, Stair, StairNode};
use richmond_building_components::{BuildingComponents, Layers, Placement};

use crate::stair_flights::FlightPolyline;

/// Inputs for fitting a spiral to two walk-ons (avoids a `connecting` cycle).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralFlightFit {
	pub lower_walk_on: Vec3,
	pub upper_walk_on: Vec3,
	/// Outward XZ into the well at the lower opening.
	pub lower_out: bevy_math::Vec2,
	pub lower_width: f32,
	pub upper_width: f32,
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

	/// Fit a circular run so the first tread sits at the lower walk-on.
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
	let opening_w = fit.lower_width.max(fit.upper_width).max(1e-4);
	let tread_width = (opening_w * 0.45).clamp(0.35, 1.25);
	let tread_depth = (tread_width * 0.55).clamp(0.25, 0.45);

	let (center, radius) = spiral_center_radius(polyline, fit, tread_width);
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
	tread_width: f32,
) -> (Vec3, f32) {
	let a = xz(fit.lower_walk_on);
	let b = xz(fit.upper_walk_on);
	let mid = polyline.stations.get(1).map(|s| xz(s.center)).unwrap_or_else(|| (a + b) * 0.5);
	let r_a = (mid - a).length();
	let r_b = (mid - b).length();
	let mut radius = (0.5 * (r_a + r_b)).max(0.5 * tread_width + 0.35);

	let mut center = Vec3::new(mid.x, fit.lower_walk_on.y, mid.y);
	if r_a < 0.2 && r_b < 0.2 {
		// Stacked openings share plan; sit the well just inside the lower door.
		let out = normalize_xz(fit.lower_out).unwrap_or(bevy_math::Vec2::X);
		radius = (0.5 * fit.lower_width + 0.35).max(0.75);
		center = fit.lower_walk_on + Vec3::new(out.x, 0.0, out.y) * radius;
	}
	(center, radius.max(1e-4))
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

/// First spiral tread is at local \(+X\); yaw sends that to the lower walk-on.
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
	// Placement yaw: local +X → `(cos yaw, 0, -sin yaw)`.
	toward.y.atan2(-toward.x)
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
	fn fit_emits_spiral_stair_node() {
		let polyline = FlightPolyline::new([
			FlightStation { center: Vec3::new(0.0, 0.0, -3.0), height: 2.2 },
			FlightStation { center: Vec3::ZERO, height: 2.2 },
			FlightStation { center: Vec3::new(3.0, 3.0, 0.0), height: 2.2 },
		]);
		let flight = SpiralFlight::fit(
			polyline,
			SpiralFlightFit {
				lower_walk_on: Vec3::new(0.0, 0.0, -3.0),
				upper_walk_on: Vec3::new(3.0, 3.0, 0.0),
				lower_out: bevy_math::Vec2::Y,
				lower_width: 2.4,
				upper_width: 2.4,
			},
		);
		assert!(matches!(&flight.stairs().geometry, Stair::Spiral(g) if g.height > 2.9));
		assert!(!flight.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}
}
