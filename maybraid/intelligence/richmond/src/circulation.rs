//! World-space storey slabs and stairwell links, derived from Richmond IR.

use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use richmond_buildings::{ConnectingStairwell, MixedUseLesHallesStorey};

/// One Les Halles storey volume in world space.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CirculationStorey {
	pub id: u32,
	pub bounds: Aabb3d,
	pub floor_y: f32,
}

impl CirculationStorey {
	pub fn contains_xz(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.bounds.min);
		let max = Vec3::from(self.bounds.max);
		p.x >= min.x - 0.35 && p.x <= max.x + 0.35 && p.z >= min.z - 0.35 && p.z <= max.z + 0.35
	}

	pub fn contains_y(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.bounds.min);
		let max = Vec3::from(self.bounds.max);
		p.y >= min.y - 0.5 && p.y <= max.y + 0.5
	}

	pub fn contains(&self, p: Vec3) -> bool {
		self.contains_xz(p) && self.contains_y(p)
	}
}

/// One climb between storeys: mouth, tread polyline (walk surface), landing.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CirculationStairwell {
	pub from_storey: u32,
	pub to_storey: u32,
	pub well: Aabb3d,
	pub mouth: Vec3,
	pub landing: Vec3,
	/// Walk-surface points, lower storey toward upper.
	pub polyline: Vec<Vec3>,
}

impl CirculationStairwell {
	pub fn contains_actor(&self, p: Vec3) -> bool {
		let min = Vec3::from(self.well.min);
		let max = Vec3::from(self.well.max);
		p.x >= min.x - 0.6
			&& p.x <= max.x + 0.6
			&& p.z >= min.z - 0.6
			&& p.z <= max.z + 0.6
			&& p.y >= min.y - 1.0
			&& p.y <= max.y + 1.2
	}

	/// Remaining walk-surface points at or above `from` (ascending).
	pub fn remaining_ascending(&self, from: Vec3) -> Vec<Vec3> {
		if self.polyline.is_empty() {
			return vec![self.landing];
		}
		let start = nearest_point_index(&self.polyline, from);
		self.polyline[start..].to_vec()
	}

	pub fn oriented_polyline(&self, going_up: bool, from: Vec3) -> Vec<Vec3> {
		if going_up {
			self.remaining_ascending(from)
		} else {
			let mut pts = self.polyline.clone();
			pts.reverse();
			if pts.is_empty() {
				return vec![self.mouth];
			}
			let start = nearest_point_index(&pts, from);
			pts[start..].to_vec()
		}
	}
}

fn nearest_point_index(points: &[Vec3], from: Vec3) -> usize {
	points
		.iter()
		.enumerate()
		.min_by(|(_, a), (_, b)| a.distance_squared(from).total_cmp(&b.distance_squared(from)))
		.map(|(i, _)| i)
		.unwrap_or(0)
}

/// Stamp a storey host from a mixed-use Les Halles floor.
pub fn circulation_from_storey(
	id: u32,
	storey: &MixedUseLesHallesStorey,
	world: Transform,
) -> CirculationStorey {
	let plan = storey.floor_plan();
	let hx = plan.outer.x * 0.5;
	let hz = plan.outer.y * 0.5;
	let c = plan.center_xz;
	let h = plan.storey_height.max(1e-3);
	let local = Aabb3d::from_min_max(
		Vec3::new(c.x - hx, c.y, c.z - hz),
		Vec3::new(c.x + hx, c.y + h, c.z + hz),
	);
	CirculationStorey {
		id,
		bounds: transform_aabb(local, world),
		floor_y: world.transform_point(Vec3::new(c.x, c.y, c.z)).y,
	}
}

/// Stamp a stairwell link. `from_storey` is the lower floor index.
pub fn circulation_from_stairwell(
	from_storey: u32,
	to_storey: u32,
	stairwell: &ConnectingStairwell,
	world: Transform,
) -> CirculationStairwell {
	let well = stairwell.well();
	let outward = xz_to_vec3(well.walk_on.into_xz());
	let off = xz_to_vec3(well.walk_off.into_xz());
	let mouth_local = well.side_mid(well.walk_on, well.bottom_y()) + outward * 1.35;
	let landing_local = well.side_mid(well.walk_off, well.top_y()) - off * 1.1;
	let mut polyline = vec![mouth_local];
	let mut tread_points = Vec::new();
	let mut lineup = None;
	for node in stairwell.stairs() {
		for (center, rotation, size) in node.tread_cuboids() {
			let top = center + rotation * Vec3::Y * (size.y * 0.5);
			if lineup.is_none() {
				let travel = (rotation * Vec3::X).with_y(0.0).normalize_or_zero();
				let trailing = top - travel * (size.x * 0.5);
				let mut entry = trailing - travel * 0.45;
				entry.y = well.bottom_y();
				lineup = Some(entry);
			}
			tread_points.push(top);
		}
	}
	if let Some(lineup) = lineup {
		// Approach the first flight centerline before climbing. This makes the
		// route turn on the run-in slab instead of cutting across the ramp side.
		polyline.push(lineup);
	}
	polyline.extend(tread_points);
	polyline.push(landing_local);
	CirculationStairwell {
		from_storey,
		to_storey,
		well: transform_aabb(well.bounds, world),
		mouth: world.transform_point(mouth_local),
		landing: world.transform_point(landing_local),
		polyline: polyline.into_iter().map(|p| world.transform_point(p)).collect(),
	}
}

fn xz_to_vec3(v: bevy_math::Vec2) -> Vec3 {
	Vec3::new(v.x, 0.0, v.y)
}

fn transform_aabb(bounds: Aabb3d, world: Transform) -> Aabb3d {
	let corners = [
		Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
		Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
		Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
		Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
		Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
		Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
		Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
		Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
	];
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in corners {
		let w = world.transform_point(c);
		min = min.min(w);
		max = max.max(w);
	}
	Aabb3d::from_min_max(min, max)
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::panels::PanelStyle;
	use richmond_buildings::{StairwellKind, WellAabb, WellSide};

	#[test]
	fn rectangular_well_polyline_rises() -> anyhow::Result<()> {
		let well = WellAabb::from_plan(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(3.5, 3.6, 3.5),
			WellSide::NegZ,
			WellSide::NegZ,
			0.4,
		);
		let stairwell = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			well,
			StairwellKind::Rectangular,
		);
		let link = circulation_from_stairwell(0, 1, &stairwell, Transform::IDENTITY);
		assert!(link.polyline.len() >= 3, "{}", link.polyline.len());
		let first = link.polyline.first().copied().unwrap_or_default();
		let last = link.polyline.last().copied().unwrap_or_default();
		assert!(last.y > first.y + 1.0, "{} vs {}", last.y, first.y);
		assert!(link.landing.y > link.mouth.y);
		let lineup = link.polyline[1];
		let first_tread = link.polyline[2];
		let first_node = stairwell.stairs().first().expect("first flight");
		let travel = (first_node.placement.rotation() * Vec3::X).with_y(0.0).normalize();
		let entry_direction = (first_tread - lineup).with_y(0.0).normalize();
		assert!(
			entry_direction.dot(travel) > 0.99,
			"entry should align with first flight: {entry_direction} vs {travel}"
		);
		assert!((lineup.y - well.bottom_y()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn storey_contains_center() -> anyhow::Result<()> {
		let bounds = Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 3.5, 4.0));
		let storey = CirculationStorey { id: 0, bounds, floor_y: 0.0 };
		assert!(storey.contains(Vec3::new(0.0, 1.0, 0.0)));
		assert!(!storey.contains(Vec3::new(0.0, 8.0, 0.0)));
		Ok(())
	}
}
