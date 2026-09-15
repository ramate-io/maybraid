//! Kitchen remainder → wall counters, range, shelves, basin, fridge, props.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::{FurnitureAbutment, FurnitureNode, FurnitureUsageNode};

use crate::region::{
	along_is_x, along_span, depth_span, floor_height_aabb, longest_wall, sit_on_aabb, slice_along,
	stamp_make, unit, wall_strip, COUNTER_SLOT_HEIGHT,
};

const RUN_DEPTH: f32 = 0.7;
const RUN_PAD: f32 = 0.08;
const STATION: f32 = 1.5;
const SHELF_Y0: f32 = 1.55;
const SHELF_H: f32 = 0.35;
const SHELF_DEPTH: f32 = 0.32;
const RANGE_ALONG: f32 = 0.85;
const FRIDGE: Vec3 = Vec3::new(0.7, 1.8, 0.7);
const BASIN: Vec3 = Vec3::new(0.56, 0.22, 0.48);
const FAUCET: Vec3 = Vec3::new(0.18, 0.34, 0.18);
const COOKWARE: Vec3 = Vec3::new(0.36, 0.22, 0.36);

/// Expand a [`FurnitureUsage::BitesKitchen`](richmond_building_components::FurnitureUsage::BitesKitchen) remainder.
pub struct BitesKitchenUsage;

impl BitesKitchenUsage {
	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let wall = node.abutment.unwrap_or_else(|| longest_wall(&region));
		let run = wall_strip(&region, wall, RUN_DEPTH, RUN_PAD);
		let along_x = along_is_x(&run, Some(wall));
		let along = along_span(&run, along_x);
		if along < 0.6 || depth_span(&run, along_x) < 0.35 {
			return leftover_chest(&region, node);
		}

		let mut out = Vec::new();
		let n = ((along / STATION).floor() as usize).max(1);
		let station = along / n as f32;
		let range_i = range_index(n, node.finish_seed);
		let basin_i = basin_index(n, range_i, node.finish_seed);

		for i in 0..n {
			let slice = slice_along(&run, along_x, i as f32 * station, (i as f32 + 1.0) * station);
			if Some(i) == range_i {
				let range_box = center_along(&slice, along_x, RANGE_ALONG.min(station - 0.05));
				out.push(stamp_make(
					FurnitureNode::range,
					&floor_height_aabb(&range_box, COUNTER_SLOT_HEIGHT),
					&region,
					Some(wall),
				));
				continue;
			}
			let slab = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::counter, &slab, &region, Some(wall)));
			if Some(i) == basin_i {
				out.push(stamp_make(
					FurnitureNode::basin,
					&sit_on_aabb(&slab, BASIN),
					&region,
					Some(wall),
				));
				out.push(stamp_make(
					FurnitureNode::faucet,
					&sit_on_aabb(&slab, FAUCET),
					&region,
					Some(wall),
				));
			} else if unit(node.finish_seed, 20 + i as u64) > 0.45 {
				out.push(stamp_make(
					FurnitureNode::cookware,
					&sit_on_aabb(&slab, COOKWARE),
					&region,
					Some(wall),
				));
			}
		}

		let shelf = shelf_on_wall(&run, wall);
		if (shelf.max.y - shelf.min.y) > 0.2 {
			out.push(stamp_make(FurnitureNode::shelf, &shelf, &region, Some(wall)));
		}

		if let Some(fridge) = fridge_on_side(&region, wall, node.finish_seed) {
			out.push(stamp_make(FurnitureNode::fridge, &fridge, &region, None));
		}
		out
	}
}

fn range_index(n: usize, seed: u64) -> Option<usize> {
	if n < 2 {
		return None;
	}
	Some(((unit(seed, 5) * n as f32) as usize).min(n - 1))
}

fn basin_index(n: usize, range_i: Option<usize>, seed: u64) -> Option<usize> {
	if n == 0 {
		return None;
	}
	let mut i = ((unit(seed, 7) * n as f32) as usize).min(n - 1);
	if Some(i) == range_i && n > 1 {
		i = (i + 1) % n;
	}
	Some(i)
}

fn center_along(slice: &Aabb3d, along_x: bool, width: f32) -> Aabb3d {
	let span = along_span(slice, along_x);
	let width = width.min(span);
	let pad = ((span - width) * 0.5).max(0.0);
	slice_along(slice, along_x, pad, pad + width)
}

fn shelf_on_wall(run: &Aabb3d, wall: FurnitureAbutment) -> Aabb3d {
	let along_x = along_is_x(run, Some(wall));
	let mut strip = wall_strip(run, wall, SHELF_DEPTH.min(depth_span(run, along_x)), 0.04);
	let y0 = run.min.y + SHELF_Y0;
	strip.min.y = y0;
	strip.max.y = y0 + SHELF_H;
	strip
}

fn fridge_on_side(region: &Aabb3d, run_wall: FurnitureAbutment, seed: u64) -> Option<Aabb3d> {
	if unit(seed, 13) < 0.2 {
		return None;
	}
	let side = match run_wall {
		FurnitureAbutment::NegZ | FurnitureAbutment::PosZ => FurnitureAbutment::NegX,
		FurnitureAbutment::NegX | FurnitureAbutment::PosX => FurnitureAbutment::NegZ,
	};
	let along_x = along_is_x(region, Some(side));
	if along_span(region, along_x) < FRIDGE.x + 0.3 {
		return None;
	}
	if depth_span(region, along_x) < FRIDGE.z + 0.25 {
		return None;
	}
	let strip = wall_strip(region, side, FRIDGE.z, RUN_PAD);
	let slot = slice_along(&strip, along_x, 0.0, FRIDGE.x.min(along_span(&strip, along_x)));
	Some(floor_height_aabb(&slot, FRIDGE.y))
}

fn leftover_chest(region: &Aabb3d, node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
	let fp = Vec3::new(region.max.x - region.min.x, 0.0, region.max.z - region.min.z);
	if fp.x < 1.1 || fp.z < 1.1 || unit(node.finish_seed, 17) > 0.65 {
		return Vec::new();
	}
	let slot = Aabb3d::from_min_max(
		Vec3::new(region.min.x + 0.2, region.min.y, region.min.z + 0.2),
		Vec3::new(region.min.x + 1.1, region.min.y + 0.75, region.min.z + 0.65),
	);
	vec![stamp_make(FurnitureNode::chest, &slot, region, node.abutment)]
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::{FurnitureGeometry, Placement};

	fn kitchen_node(min: Vec3, max: Vec3) -> FurnitureUsageNode {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.5, 8.0));
		let box_ = Aabb3d::from_min_max(min, max);
		let mut node = FurnitureUsageNode::bites_kitchen(Placement::IDENTITY);
		node.stamp_region(&box_, &host);
		node
	}

	#[test]
	fn roomy_kitchen_gets_a_run_and_not_one_room_counter() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 2.0), Vec3::new(8.0, 3.5, 6.0));
		let pieces = BitesKitchenUsage::expand(&node);
		let counters: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).collect();
		assert!(!counters.is_empty());
		for c in &counters {
			let along = c.placement.scale.x.max(c.placement.scale.z);
			assert!(along < 6.5, "kitchen counter should be a station, got {along}");
		}
		assert!(pieces
			.iter()
			.any(|n| matches!(n.geometry, FurnitureGeometry::Range | FurnitureGeometry::Shelf)));
	}

	#[test]
	fn tight_kitchen_does_not_panic() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.2, 3.0, 1.2));
		let _ = BitesKitchenUsage::expand(&node);
	}
}
