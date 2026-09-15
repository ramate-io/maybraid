//! Sit-down pocket → cafe tables (island counters) and chairs around them.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::furniture::abutment::local_scale_for_yaw;
use richmond_building_components::{FurnitureNode, FurnitureUsageNode, Placement};

use crate::region::{along_is_x, along_span, depth_span, floor_height_aabb, stamp_make, unit};

const TABLE: Vec3 = Vec3::new(1.15, 0.75, 0.70);
const CHAIR: Vec3 = Vec3::new(0.46, 0.82, 0.46);
const CHAIR_GAP: f32 = 0.12;
const CELL_PAD: f32 = 0.20;

/// Expand a [`FurnitureUsage::BitesSeating`](richmond_building_components::FurnitureUsage::BitesSeating) pocket.
pub struct BitesSeatingUsage;

impl BitesSeatingUsage {
	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let along_x = along_is_x(&region, node.abutment);
		let along = along_span(&region, along_x);
		let depth = depth_span(&region, along_x);
		if along < 1.2 || depth < 1.2 {
			return spare_chairs(&region, node);
		}

		let cell_along = TABLE.x + 0.55;
		let cell_depth = TABLE.z + 2.0 * (CHAIR.z + CHAIR_GAP) + CELL_PAD;
		let n_along = ((along / cell_along).floor() as usize).max(1);
		let n_depth = ((depth / cell_depth).floor() as usize).max(1);
		let used_a = n_along as f32 * cell_along;
		let used_d = n_depth as f32 * cell_depth;
		let a0 = ((along - used_a) * 0.5).max(0.08);
		let d0 = ((depth - used_d) * 0.5).max(0.08);

		let mut out = Vec::new();
		for j in 0..n_depth {
			for i in 0..n_along {
				if unit(node.finish_seed, 30 + (j * 8 + i) as u64) < 0.08 && n_along * n_depth > 2 {
					continue;
				}
				let cell = cell_box(
					&region,
					along_x,
					a0 + i as f32 * cell_along,
					a0 + (i as f32 + 1.0) * cell_along,
					d0 + j as f32 * cell_depth,
					d0 + (j as f32 + 1.0) * cell_depth,
				);
				out.extend(table_set(&cell, &region, along_x, node.finish_seed, i + j * 5));
			}
		}
		if out.is_empty() {
			return spare_chairs(&region, node);
		}
		out
	}
}

fn table_set(
	cell: &Aabb3d,
	host: &Aabb3d,
	along_x: bool,
	seed: u64,
	salt: usize,
) -> Vec<FurnitureNode> {
	let c = (cell.min + cell.max) * 0.5;
	let table = Aabb3d::from_min_max(
		Vec3::new(c.x - TABLE.x * 0.5, cell.min.y, c.z - TABLE.z * 0.5),
		Vec3::new(c.x + TABLE.x * 0.5, cell.min.y + TABLE.y, c.z + TABLE.z * 0.5),
	);
	let table = if along_x {
		table
	} else {
		Aabb3d::from_min_max(
			Vec3::new(c.x - TABLE.z * 0.5, cell.min.y, c.z - TABLE.x * 0.5),
			Vec3::new(c.x + TABLE.z * 0.5, cell.min.y + TABLE.y, c.z + TABLE.x * 0.5),
		)
	};
	let mut out = vec![stamp_make(FurnitureNode::counter, &table, host, None)];
	let offset = TABLE.z * 0.5 + CHAIR.z * 0.5 + CHAIR_GAP;
	let sides = if along_x {
		[(0.0, -offset, 0.0_f32), (0.0, offset, std::f32::consts::PI)]
	} else {
		[(-offset, 0.0, std::f32::consts::FRAC_PI_2), (offset, 0.0, -std::f32::consts::FRAC_PI_2)]
	};
	for (k, (dx, dz, yaw)) in sides.into_iter().enumerate() {
		if unit(seed, 50 + salt as u64 + k as u64) < 0.12 {
			continue;
		}
		let chair = Aabb3d::from_min_max(
			Vec3::new(c.x + dx - CHAIR.x * 0.5, cell.min.y, c.z + dz - CHAIR.z * 0.5),
			Vec3::new(c.x + dx + CHAIR.x * 0.5, cell.min.y + CHAIR.y, c.z + dz + CHAIR.z * 0.5),
		);
		out.push(stamp_facing(FurnitureNode::chair, &chair, host, yaw));
	}
	out
}

fn stamp_facing(
	make: fn(Placement) -> FurnitureNode,
	slot: &Aabb3d,
	host: &Aabb3d,
	yaw: f32,
) -> FurnitureNode {
	let mut node = stamp_make(make, slot, host, None);
	node.abutment = None;
	node.placement.yaw = yaw;
	node.placement.scale = local_scale_for_yaw(slot, yaw);
	node
}

fn cell_box(region: &Aabb3d, along_x: bool, a0: f32, a1: f32, d0: f32, d1: f32) -> Aabb3d {
	if along_x {
		Aabb3d::from_min_max(
			Vec3::new(region.min.x + a0, region.min.y, region.min.z + d0),
			Vec3::new(region.min.x + a1, region.max.y, region.min.z + d1),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(region.min.x + d0, region.min.y, region.min.z + a0),
			Vec3::new(region.min.x + d1, region.max.y, region.min.z + a1),
		)
	}
}

fn spare_chairs(region: &Aabb3d, node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
	let slab = floor_height_aabb(region, CHAIR.y);
	let c = (slab.min + slab.max) * 0.5;
	let slot = Aabb3d::from_min_max(
		Vec3::new(c.x - CHAIR.x * 0.5, slab.min.y, c.z - CHAIR.z * 0.5),
		Vec3::new(c.x + CHAIR.x * 0.5, slab.min.y + CHAIR.y, c.z + CHAIR.z * 0.5),
	);
	vec![stamp_make(FurnitureNode::chair, &slot, region, node.abutment)]
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::{FurnitureGeometry, Placement};

	fn seating_node(min: Vec3, max: Vec3) -> FurnitureUsageNode {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.5, 8.0));
		let box_ = Aabb3d::from_min_max(min, max);
		let mut node = FurnitureUsageNode::bites_seating(Placement::IDENTITY);
		node.stamp_region(&box_, &host);
		node
	}

	#[test]
	fn roomy_pocket_gets_tables_and_chairs() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(6.0, 3.2, 4.5));
		let pieces = BitesSeatingUsage::expand(&node);
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Counter).count();
		let chairs = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count();
		assert!(tables >= 2, "expected cafe tables, got {tables}");
		assert!(chairs >= 4, "expected chairs around tables, got {chairs}");
	}

	#[test]
	fn tight_pocket_still_gets_a_chair() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.3, 3.0, 1.3));
		let pieces = BitesSeatingUsage::expand(&node);
		assert!(pieces.iter().any(|n| n.geometry == FurnitureGeometry::Chair));
	}
}
