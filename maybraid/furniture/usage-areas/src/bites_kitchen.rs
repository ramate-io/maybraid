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
const SHELF_PLAN: f32 = 2.0;
const SHELF_DECK: f32 = 0.75;
const SHELF_DECKS: usize = 5;
const SHELF_GAP: f32 = 1.5;

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

		out.extend(shelf_aisles(&interior, &region));
		out
	}
}

fn shelf_aisles(floor: &Aabb3d, host: &Aabb3d) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx < SHELF_PLAN + 0.35 || sz < SHELF_PLAN + 0.35 {
		return Vec::new();
	}
	let pitch = SHELF_PLAN + SHELF_GAP;
	let n_x = count_units(sx, SHELF_PLAN, SHELF_GAP, 2);
	let n_z = count_units(sz, SHELF_PLAN, SHELF_GAP, 2);
	if n_x == 0 || n_z == 0 {
		return Vec::new();
	}
	let used_x = n_x as f32 * SHELF_PLAN + n_x.saturating_sub(1) as f32 * SHELF_GAP;
	let used_z = n_z as f32 * SHELF_PLAN + n_z.saturating_sub(1) as f32 * SHELF_GAP;
	let x0 = floor.min.x + ((sx - used_x) * 0.5).max(0.15);
	let z0 = floor.min.z + ((sz - used_z) * 0.5).max(0.15);
	let decks = ((floor.max.y - floor.min.y - 0.15) / SHELF_DECK).floor() as usize;
	let decks = decks.clamp(2, SHELF_DECKS);
	let mut out = Vec::new();
	for iz in 0..n_z {
		for ix in 0..n_x {
			let x = x0 + ix as f32 * pitch;
			let z = z0 + iz as f32 * pitch;
			for deck in 0..decks {
				let y0 = floor.min.y + deck as f32 * SHELF_DECK;
				let slot = Aabb3d::from_min_max(
					Vec3::new(x, y0, z),
					Vec3::new(x + SHELF_PLAN, y0 + SHELF_DECK, z + SHELF_PLAN),
				);
				out.push(stamp_make(FurnitureNode::shelf, &slot, host, None));
			}
		}
	}
	out
}

fn count_units(span: f32, plan: f32, gap: f32, max: usize) -> usize {
	if span < plan + 0.3 {
		return 0;
	}
	let n = ((span + gap) / (plan + gap)).floor() as usize;
	n.clamp(1, max)
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
		let shelves: Vec<_> =
			pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Shelf).collect();
		assert!(shelves.len() >= 2, "stacked shelf rows, got {}", shelves.len());
		assert!(shelves.len() <= 20, "a few 2 m towers, got {}", shelves.len());
		for s in &shelves {
			let px = s.placement.scale.x;
			let pz = s.placement.scale.z;
			assert!((px - pz).abs() < 0.15, "shelf XZ should stay square, got {px} x {pz}");
			assert!((s.placement.scale.y - SHELF_DECK).abs() < 0.05);
		}
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
