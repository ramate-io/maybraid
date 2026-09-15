//! Kitchen remainder → wall counters, sit-on cooktop, mid-room shelves, fridge.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::{FurnitureAbutment, FurnitureNode, FurnitureUsageNode};

use crate::region::{
	along_is_x, along_span, cut_from_wall, depth_span, floor_height_aabb, longest_wall,
	sit_on_aabb, slice_along, stamp_make, stamp_make_yaw, unit, wall_strip, yaw_turns,
	COUNTER_SLOT_HEIGHT,
};

const RUN_DEPTH: f32 = 0.75;
const RUN_PAD: f32 = 0.08;
const STATION: f32 = 1.45;
const COOKTOP: Vec3 = Vec3::new(0.98, 0.16, 0.64);
const FRIDGE: Vec3 = Vec3::new(0.78, 1.85, 0.72);
const BASIN: Vec3 = Vec3::new(0.82, 0.32, 0.64);
const FAUCET: Vec3 = Vec3::new(0.24, 0.44, 0.22);
const COOKWARE: Vec3 = Vec3::new(0.42, 0.26, 0.42);
const SHELF_BAY: f32 = 1.45;
const SHELF_DEPTH: f32 = 0.42;
const SHELF_AISLE: f32 = 0.90;
const SHELF_DECK: f32 = 0.38;
const SHELF_DECKS: usize = 3;

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
		let range_i = ((unit(node.finish_seed, 5) * n as f32) as usize).min(n - 1);
		let basin_i = {
			let mut i = ((unit(node.finish_seed, 7) * n as f32) as usize).min(n - 1);
			if i == range_i && n > 1 {
				i = (i + 1) % n;
			}
			i
		};

		for i in 0..n {
			let slice = slice_along(&run, along_x, i as f32 * station, (i as f32 + 1.0) * station);
			let slab = floor_height_aabb(&slice, COUNTER_SLOT_HEIGHT);
			out.push(stamp_make(FurnitureNode::counter, &slab, &region, Some(wall)));
			if i == range_i {
				let cooktop = sit_on_aabb(&slab, COOKTOP);
				out.push(stamp_make(FurnitureNode::range, &cooktop, &region, Some(wall)));
				out.push(stamp_make_yaw(
					FurnitureNode::cookware,
					&sit_on_aabb(&cooktop, COOKWARE),
					&region,
					Some(wall),
					yaw_turns(node.finish_seed, 21 + i as u64),
				));
				if station > 1.35 {
					let mut extra = sit_on_aabb(&cooktop, COOKWARE);
					extra.min.x += 0.22;
					extra.max.x += 0.22;
					out.push(stamp_make_yaw(
						FurnitureNode::cookware,
						&extra,
						&region,
						Some(wall),
						yaw_turns(node.finish_seed, 21 + i as u64) + std::f32::consts::FRAC_PI_2,
					));
				}
			} else if i == basin_i {
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
			}
		}

		let mut interior = cut_from_wall(&region, wall, RUN_DEPTH + 0.22);
		if let Some((slot, side)) = fridge_on_wall(&region, wall) {
			out.push(stamp_make(FurnitureNode::fridge, &slot, &region, Some(side)));
			interior = cut_from_wall(&interior, side, FRIDGE.z + 0.12);
		} else if along >= FRIDGE.x + STATION {
			let width = FRIDGE.x;
			let t0 = if range_i + 1 == n { 0.0 } else { along - width };
			let slice = slice_along(&run, along_x, t0, t0 + width);
			out.push(stamp_make(
				FurnitureNode::fridge,
				&floor_height_aabb(&slice, FRIDGE.y),
				&region,
				Some(wall),
			));
		}

		out.extend(shelf_aisles(&interior, &region, node.finish_seed));
		out
	}
}

fn shelf_aisles(floor: &Aabb3d, host: &Aabb3d, seed: u64) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx < 0.85 || sz < 0.85 {
		return Vec::new();
	}
	let along_x = sx + 1e-4 >= sz;
	let along = if along_x { sx } else { sz };
	let depth = if along_x { sz } else { sx };
	let pitch = SHELF_DEPTH + SHELF_AISLE;
	let n_rows = ((depth + SHELF_AISLE) / pitch).floor() as usize;
	let n_rows = n_rows.max(if depth >= SHELF_DEPTH + 0.12 { 1 } else { 0 });
	if n_rows == 0 {
		return Vec::new();
	}
	let n_bays = ((along / SHELF_BAY).floor() as usize).max(1);
	let bay = along / n_bays as f32;
	let used_d = n_rows as f32 * SHELF_DEPTH + (n_rows.saturating_sub(1) as f32) * SHELF_AISLE;
	let d0 = ((depth - used_d) * 0.5).max(0.04);
	let decks = ((floor.max.y - floor.min.y - 0.25) / SHELF_DECK).floor() as usize;
	let decks = decks.clamp(1, SHELF_DECKS);
	let mut out = Vec::new();
	for row in 0..n_rows {
		let d_off = d0 + row as f32 * pitch;
		for bay_i in 0..n_bays {
			let a0 = bay_i as f32 * bay + 0.04;
			let a1 = (bay_i as f32 + 1.0) * bay - 0.04;
			if a1 - a0 < 0.55 {
				continue;
			}
			for deck in 0..decks {
				let y0 = floor.min.y + deck as f32 * SHELF_DECK;
				let slot = aisle_slot(floor, along_x, a0, a1, d_off, d_off + SHELF_DEPTH, y0);
				if unit(seed, 80 + (row * 11 + bay_i * 3 + deck) as u64) < 0.04 {
					continue;
				}
				out.push(stamp_make(FurnitureNode::shelf, &slot, host, None));
			}
		}
	}
	out
}

fn aisle_slot(
	floor: &Aabb3d,
	along_x: bool,
	a0: f32,
	a1: f32,
	d0: f32,
	d1: f32,
	y0: f32,
) -> Aabb3d {
	if along_x {
		Aabb3d::from_min_max(
			Vec3::new(floor.min.x + a0, y0, floor.min.z + d0),
			Vec3::new(floor.min.x + a1, y0 + SHELF_DECK, floor.min.z + d1),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(floor.min.x + d0, y0, floor.min.z + a0),
			Vec3::new(floor.min.x + d1, y0 + SHELF_DECK, floor.min.z + a1),
		)
	}
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
	matches!(a, FurnitureAbutment::NegZ | FurnitureAbutment::PosZ)
		!= matches!(b, FurnitureAbutment::NegZ | FurnitureAbutment::PosZ)
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
	fn roomy_kitchen_gets_cooktop_aisles_fridge_and_basin() {
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
		let range = pieces.iter().find(|n| n.geometry == FurnitureGeometry::Range).unwrap();
		let counter_top = counters[0].placement.translation.y + counters[0].placement.scale.y * 0.5;
		assert!(
			range.placement.translation.y > counter_top - 0.05,
			"cooktop should sit on the counter, range y {} vs top {counter_top}",
			range.placement.translation.y
		);
		let yaws: Vec<_> = pieces
			.iter()
			.filter(|n| n.geometry == FurnitureGeometry::Cookware)
			.map(|n| n.placement.yaw)
			.collect();
		if yaws.len() > 1 {
			assert!(
				yaws.iter().any(|y| (y - yaws[0]).abs() > 0.2),
				"cookware should not share one yaw, got {yaws:?}"
			);
		}
	}

	#[test]
	fn short_run_still_gets_a_cooktop() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.2, 3.5, 2.4));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "short run range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Counter), "short run counter {got:?}");
	}

	#[test]
	fn shallow_run_still_gets_a_cooktop() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(7.0, 3.5, 1.35));
		let pieces = BitesKitchenUsage::expand(&node);
		let got = kinds(&pieces);
		assert!(got.contains(&FurnitureGeometry::Range), "shallow range {got:?}");
		assert!(got.contains(&FurnitureGeometry::Fridge), "shallow fridge {got:?}");
	}

	#[test]
	fn tight_kitchen_does_not_panic() {
		let node = kitchen_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.2, 3.0, 1.2));
		let _ = BitesKitchenUsage::expand(&node);
	}
}
