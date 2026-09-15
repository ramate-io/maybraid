//! Passage-face band → station counters + leftover sit-on display.

use bevy::math::Vec3;
use richmond_building_components::{FurnitureNode, FurnitureUsageNode};

use crate::region::{
	along_is_x, along_span, floor_height_aabb, sit_on_aabb, slice_along, stamp_make, unit,
	COUNTER_SLOT_HEIGHT,
};

/// Target station width along a service band (metres).
const STATION: f32 = 1.4;
const STATION_MIN: f32 = 1.1;
const DISPLAY_MIN: f32 = 0.55;
const DISPLAY_SIZE: Vec3 = Vec3::new(0.70, 0.42, 0.50);
const FOOD_SIZE: Vec3 = Vec3::new(0.28, 0.22, 0.28);

/// Expand a [`FurnitureUsage::BitesCounter`](richmond_building_components::FurnitureUsage::BitesCounter) band.
pub struct BitesCounterUsage;

impl BitesCounterUsage {
	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let along_x = along_is_x(&region, node.abutment);
		let along = along_span(&region, along_x);
		if along < 0.4 {
			return Vec::new();
		}

		let (stations, leftover_start) = split_stations(along, node.finish_seed);
		let mut out = Vec::new();
		let mut cursor = 0.0;
		for i in 0..stations {
			let end = if i + 1 == stations && leftover_start.is_none() {
				along
			} else {
				((i as f32 + 1.0) * (leftover_start.unwrap_or(along) / stations as f32)).min(along)
			};
			let slice = slice_along(&region, along_x, cursor, end);
			let slot = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::counter, &slot, &region, node.abutment));
			cursor = end;
		}

		if let Some(start) = leftover_start {
			let leftover = slice_along(&region, along_x, start, along);
			let slab = floor_height_aabb(&leftover, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::counter, &slab, &region, node.abutment));
			out.extend(sit_ons_on(&slab, &region, node));
		} else if let Some(last) = out.last() {
			let slab = last_slab(last);
			out.extend(sit_ons_on(&slab, &region, node));
		}
		out
	}
}

fn split_stations(along: f32, seed: u64) -> (usize, Option<f32>) {
	if along < STATION_MIN * 2.0 - 0.15 {
		return (1, None);
	}
	let n = ((along / STATION).floor() as usize).max(1);
	let used = STATION * n as f32;
	let leftover = along - used;
	if leftover >= DISPLAY_MIN && unit(seed, 3) > 0.25 {
		(n, Some(along - leftover.max(DISPLAY_MIN).min(0.9)))
	} else {
		(n.max(1), None)
	}
}

fn last_slab(node: &FurnitureNode) -> bevy::math::bounding::Aabb3d {
	let c = node.placement.translation;
	let half = node.placement.scale * 0.5;
	// Local scale may swap XZ under yaw; recover a conservative world slab.
	let hx = half.x.max(half.z);
	let hz = half.x.min(half.z).max(0.15);
	floor_height_aabb(
		&bevy::math::bounding::Aabb3d::from_min_max(
			Vec3::new(c.x - hx, c.y - half.y, c.z - hz),
			Vec3::new(c.x + hx, c.y + half.y, c.z + hz),
		),
		COUNTER_SLOT_HEIGHT,
	)
}

fn sit_ons_on(
	slab: &bevy::math::bounding::Aabb3d,
	host: &bevy::math::bounding::Aabb3d,
	node: &FurnitureUsageNode,
) -> Vec<FurnitureNode> {
	let pick = unit(node.finish_seed, 11);
	let mut out = vec![stamp_make(
		FurnitureNode::food_display,
		&sit_on_aabb(slab, DISPLAY_SIZE),
		host,
		node.abutment,
	)];
	if pick > 0.35 {
		let food = if pick > 0.7 { FurnitureNode::fruit } else { FurnitureNode::bread };
		let mut food_slot = sit_on_aabb(slab, FOOD_SIZE);
		food_slot.min.x += 0.18;
		food_slot.max.x += 0.18;
		out.push(stamp_make(food, &food_slot, host, node.abutment));
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use richmond_building_components::{FurnitureGeometry, FurnitureUsageNode, Placement};

	fn band_node(min: Vec3, max: Vec3) -> FurnitureUsageNode {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.5, 6.0));
		let band = Aabb3d::from_min_max(min, max);
		let mut node = FurnitureUsageNode::bites_counter(Placement::IDENTITY);
		node.stamp_region(&band, &host);
		node
	}

	#[test]
	fn long_band_is_stations_not_one_stretched_counter() {
		let node = band_node(Vec3::new(1.0, 0.0, 0.0), Vec3::new(6.2, 3.5, 0.8));
		let pieces = BitesCounterUsage::expand(&node);
		let counters: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).collect();
		assert!(counters.len() >= 2, "expected stations, got {}", counters.len());
		for c in &counters {
			let along = c.placement.scale.x.max(c.placement.scale.z);
			assert!(along < 4.5, "station should not fill the 5.2 m band, got {along}");
			assert!((c.placement.scale.y - COUNTER_SLOT_HEIGHT).abs() < 1e-3);
		}
		assert!(pieces.iter().any(|n| n.geometry == FurnitureGeometry::FoodDisplay));
	}

	#[test]
	fn short_band_still_gets_a_counter() {
		let node = band_node(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.2, 3.5, 0.8));
		let pieces = BitesCounterUsage::expand(&node);
		assert!(pieces.iter().any(|n| n.geometry == FurnitureGeometry::Counter));
	}

	#[test]
	fn east_wall_band_keeps_depth_on_x() {
		let node = band_node(Vec3::new(5.2, 0.0, 1.0), Vec3::new(6.0, 3.5, 5.0));
		let pieces = BitesCounterUsage::expand(&node);
		let counters: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).collect();
		assert!(!counters.is_empty());
		for c in counters {
			assert!(
				(c.placement.scale.z - 0.8).abs() < 0.05
					|| (c.placement.scale.x - 0.8).abs() < 0.05
			);
		}
	}
}
