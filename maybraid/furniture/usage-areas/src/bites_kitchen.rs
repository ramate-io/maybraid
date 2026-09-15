//! Kitchen remainder → wall counters, range, shelves, basin, fridge, props.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::{FurnitureAbutment, FurnitureNode, FurnitureUsageNode};

use crate::region::{
	along_is_x, along_span, depth_span, floor_height_aabb, longest_wall, sit_on_aabb, slice_along,
	stamp_make, unit, wall_strip, COUNTER_SLOT_HEIGHT,
};

const RUN_DEPTH: f32 = 0.75;
const RUN_PAD: f32 = 0.08;
const STATION: f32 = 1.45;
const SHELF_Y0: [f32; 2] = [1.48, 1.96];
const SHELF_H: f32 = 0.40;
const SHELF_DEPTH: f32 = 0.40;
const RANGE_ALONG: f32 = 1.10;
const RANGE_MIN: f32 = 0.85;
const FRIDGE: Vec3 = Vec3::new(0.78, 1.85, 0.72);
const BASIN: Vec3 = Vec3::new(0.82, 0.32, 0.64);
const FAUCET: Vec3 = Vec3::new(0.24, 0.44, 0.22);
const COOKWARE: Vec3 = Vec3::new(0.54, 0.30, 0.54);
const RANGE_COOKWARE: Vec3 = Vec3::new(0.44, 0.24, 0.44);

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
		let mut lo = 0.0;
		let mut hi = along;
		let range_at_hi = unit(node.finish_seed, 5) > 0.5 && along > RANGE_ALONG + 0.55;

		if along >= RANGE_MIN {
			let width = RANGE_ALONG.min(along);
			let t0 = if range_at_hi { along - width } else { 0.0 };
			let slice = slice_along(&run, along_x, t0, t0 + width);
			let slab = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::range, &slab, &region, Some(wall)));
			out.push(stamp_make(
				FurnitureNode::cookware,
				&sit_on_aabb(&slab, RANGE_COOKWARE),
				&region,
				Some(wall),
			));
			if range_at_hi {
				hi -= width;
			} else {
				lo += width;
			}
		}

		if let Some((slot, side)) = fridge_on_wall(&region, wall) {
			out.push(stamp_make(FurnitureNode::fridge, &slot, &region, Some(side)));
		} else if hi - lo >= FRIDGE.x + 0.50 {
			let width = FRIDGE.x.min(hi - lo);
			let t0 = if range_at_hi { lo } else { hi - width };
			let slice = slice_along(&run, along_x, t0, t0 + width);
			out.push(stamp_make(
				FurnitureNode::fridge,
				&floor_height_aabb(&slice, FRIDGE.y),
				&region,
				Some(wall),
			));
			if range_at_hi {
				lo += width;
			} else {
				hi -= width;
			}
		}

		let remain = hi - lo;
		if remain >= 0.55 {
			let n = ((remain / STATION).floor() as usize).max(1);
			let station = remain / n as f32;
			let basin_i = ((unit(node.finish_seed, 7) * n as f32) as usize).min(n - 1);
			for i in 0..n {
				let t0 = lo + i as f32 * station;
				let slice = slice_along(&run, along_x, t0, t0 + station);
				let slab = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
				out.push(stamp_make(FurnitureNode::counter, &slab, &region, Some(wall)));
				if i == basin_i {
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
				} else if unit(node.finish_seed, 20 + i as u64) > 0.28 {
					out.push(stamp_make(
						FurnitureNode::cookware,
						&sit_on_aabb(&slab, COOKWARE),
						&region,
						Some(wall),
					));
				}
			}
		}

		out.extend(shelves_on_run(&run, wall, &region));
		out
	}
}

fn shelves_on_run(run: &Aabb3d, wall: FurnitureAbutment, region: &Aabb3d) -> Vec<FurnitureNode> {
	let along_x = along_is_x(run, Some(wall));
	let depth = SHELF_DEPTH.min(depth_span(run, along_x));
	if depth < 0.18 {
		return Vec::new();
	}
	let mut out = Vec::new();
	let ceiling = region.max.y - region.min.y;
	for y0 in SHELF_Y0 {
		if ceiling < y0 + SHELF_H + 0.12 {
			continue;
		}
		let mut strip = wall_strip(run, wall, depth, 0.04);
		strip.min.y = run.min.y + y0;
		strip.max.y = strip.min.y + SHELF_H;
		if strip.max.y - strip.min.y > 0.2 {
			out.push(stamp_make(FurnitureNode::shelf, &strip, region, Some(wall)));
		}
	}
	out
}

fn fridge_on_wall(
	region: &Aabb3d,
	run_wall: FurnitureAbutment,
) -> Option<(Aabb3d, FurnitureAbutment)> {
	for side in fridge_walls(run_wall) {
		if let Some(slot) = fridge_against(region, run_wall, side) {
			return Some((slot, side));
		}
	}
	None
}

fn fridge_walls(run: FurnitureAbutment) -> [FurnitureAbutment; 3] {
	match run {
		FurnitureAbutment::NegZ => {
			[FurnitureAbutment::NegX, FurnitureAbutment::PosX, FurnitureAbutment::PosZ]
		}
		FurnitureAbutment::PosZ => {
			[FurnitureAbutment::PosX, FurnitureAbutment::NegX, FurnitureAbutment::NegZ]
		}
		FurnitureAbutment::NegX => {
			[FurnitureAbutment::NegZ, FurnitureAbutment::PosZ, FurnitureAbutment::PosX]
		}
		FurnitureAbutment::PosX => {
			[FurnitureAbutment::PosZ, FurnitureAbutment::NegZ, FurnitureAbutment::NegX]
		}
	}
}

fn fridge_against(
	region: &Aabb3d,
	run_wall: FurnitureAbutment,
	side: FurnitureAbutment,
) -> Option<Aabb3d> {
	let along_x = along_is_x(region, Some(side));
	let inset = if perpendicular(run_wall, side) { RUN_DEPTH + 0.12 } else { RUN_PAD };
	if along_span(region, along_x) < inset + FRIDGE.x + 0.08 {
		return None;
	}
	let depth = depth_span(region, along_x);
	if depth < FRIDGE.z + 0.22 {
		return None;
	}
	if !perpendicular(run_wall, side) && depth - RUN_DEPTH < FRIDGE.z + 0.08 {
		return None;
	}
	let strip = wall_strip(region, side, FRIDGE.z, RUN_PAD);
	let slot = slice_along(&strip, along_x, inset, inset + FRIDGE.x);
	Some(floor_height_aabb(&slot, FRIDGE.y))
}

fn perpendicular(a: FurnitureAbutment, b: FurnitureAbutment) -> bool {
	along_is_x_wall(a) != along_is_x_wall(b)
}

fn along_is_x_wall(wall: FurnitureAbutment) -> bool {
	matches!(wall, FurnitureAbutment::NegZ | FurnitureAbutment::PosZ)
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

	fn kinds(pieces: &[FurnitureNode]) -> Vec<FurnitureGeometry> {
		pieces.iter().map(|n| n.geometry).collect()
	}

	#[test]
	fn roomy_kitchen_gets_range_shelf_fridge_and_basin() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 2.0), Vec3::new(8.0, 3.5, 6.0));
		let pieces = BitesKitchenUsage::expand(&node);
		let counters: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).collect();
		assert!(!counters.is_empty());
		for c in &counters {
			let along = c.placement.scale.x.max(c.placement.scale.z);
			assert!(along < 6.5, "kitchen counter should be a station, got {along}");
		}
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Shelf), "shelf {got:?}");
		assert!(got.contains(&FurnitureGeometry::Fridge), "fridge {got:?}");
		assert!(got.contains(&FurnitureGeometry::Basin), "basin {got:?}");
		assert!(got.contains(&FurnitureGeometry::Cookware), "cookware {got:?}");
	}

	#[test]
	fn short_run_still_gets_a_range() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.2, 3.5, 2.4));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "short run range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Shelf), "short run shelf {got:?}");
	}

	#[test]
	fn shallow_run_parks_fridge_on_the_run() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(7.0, 3.5, 1.35));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "shallow range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Fridge), "shallow fridge {got:?}");
		assert!(got.contains(&FurnitureGeometry::Shelf), "shallow shelf {got:?}");
	}

	#[test]
	fn tight_kitchen_does_not_panic() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.2, 3.0, 1.2));
		let _ = BitesKitchenUsage::expand(&node);
	}
}
