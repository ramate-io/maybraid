//! Sit-down pocket → cafe tables and chairs fitted into the packed box.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::furniture::abutment::local_scale_for_yaw;
use richmond_building_components::{FurnitureNode, FurnitureUsageNode, Placement};

use crate::region::{along_is_x, along_span, depth_span, floor_height_aabb, stamp_make};

const TABLE: Vec3 = Vec3::new(1.20, 0.75, 0.80);
const CHAIR: Vec3 = Vec3::new(0.46, 0.82, 0.46);
const CHAIR_GAP: f32 = 0.14;
const CELL_ALONG: f32 = 2.5;
const CELL_DEPTH: f32 = 2.3;
const MAX_TABLES: usize = 4;

/// Expand a [`FurnitureUsage::BitesSeating`](richmond_building_components::FurnitureUsage::BitesSeating) pocket.
pub struct BitesSeatingUsage;

impl BitesSeatingUsage {
	pub fn expand(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
		let region = node.region_aabb();
		let along_x = along_is_x(&region, node.abutment);
		let along = along_span(&region, along_x);
		let depth = depth_span(&region, along_x);
		if along < 1.5 || depth < 1.5 {
			return spare_chairs(&region, node);
		}

		let n_along = count_cells(along, CELL_ALONG, MAX_TABLES).max(1);
		let n_depth = count_cells(depth, CELL_DEPTH, 2).max(1);
		let mut n_along = n_along;
		if n_along * n_depth > MAX_TABLES {
			n_along = (MAX_TABLES / n_depth).max(1);
		}
		let cell_a = along / n_along as f32;
		let cell_d = depth / n_depth as f32;

		let mut out = Vec::new();
		for j in 0..n_depth {
			for i in 0..n_along {
				let cell = cell_box(
					&region,
					along_x,
					i as f32 * cell_a,
					(i as f32 + 1.0) * cell_a,
					j as f32 * cell_d,
					(j as f32 + 1.0) * cell_d,
				);
				out.extend(table_set(&cell, &region, along_x));
			}
		}
		if out.is_empty() {
			return spare_chairs(&region, node);
		}
		out
	}
}

fn count_cells(span: f32, target: f32, max: usize) -> usize {
	((span / target).floor() as usize).clamp(0, max)
}

fn table_set(cell: &Aabb3d, host: &Aabb3d, along_x: bool) -> Vec<FurnitureNode> {
	let c = (cell.min + cell.max) * 0.5;
	let (hx, hz) =
		if along_x { (TABLE.x * 0.5, TABLE.z * 0.5) } else { (TABLE.z * 0.5, TABLE.x * 0.5) };
	let table = clamp_xz(
		Aabb3d::from_min_max(
			Vec3::new(c.x - hx, cell.min.y, c.z - hz),
			Vec3::new(c.x + hx, cell.min.y + TABLE.y, c.z + hz),
		),
		cell,
	);
	if (table.max.x - table.min.x) < 0.7 || (table.max.z - table.min.z) < 0.55 {
		return spare_chairs(cell, &dummy_node());
	}
	let mut out = vec![stamp_make(FurnitureNode::table, &table, host, None)];
	let tc = (table.min + table.max) * 0.5;
	let offset_z = (table.max.z - table.min.z) * 0.5 + CHAIR.z * 0.5 + CHAIR_GAP;
	let offset_x = (table.max.x - table.min.x) * 0.5 + CHAIR.x * 0.5 + CHAIR_GAP;
	let sides = if along_x {
		[(0.0, -offset_z, 0.0_f32), (0.0, offset_z, std::f32::consts::PI)]
	} else {
		[
			(-offset_x, 0.0, std::f32::consts::FRAC_PI_2),
			(offset_x, 0.0, -std::f32::consts::FRAC_PI_2),
		]
	};
	for (dx, dz, yaw) in sides {
		let chair = Aabb3d::from_min_max(
			Vec3::new(tc.x + dx - CHAIR.x * 0.5, cell.min.y, tc.z + dz - CHAIR.z * 0.5),
			Vec3::new(tc.x + dx + CHAIR.x * 0.5, cell.min.y + CHAIR.y, tc.z + dz + CHAIR.z * 0.5),
		);
		if !contains_xz(cell, &chair) {
			continue;
		}
		out.push(stamp_facing(FurnitureNode::chair, &chair, host, yaw));
	}
	out
}

fn dummy_node() -> FurnitureUsageNode {
	FurnitureUsageNode::bites_seating(Placement::IDENTITY)
}

fn clamp_xz(slot: Aabb3d, host: &Aabb3d) -> Aabb3d {
	let pad = 0.06;
	Aabb3d::from_min_max(
		Vec3::new(slot.min.x.max(host.min.x + pad), slot.min.y, slot.min.z.max(host.min.z + pad)),
		Vec3::new(slot.max.x.min(host.max.x - pad), slot.max.y, slot.max.z.min(host.max.z - pad)),
	)
}

fn contains_xz(host: &Aabb3d, slot: &Aabb3d) -> bool {
	slot.min.x >= host.min.x - 1e-3
		&& slot.max.x <= host.max.x + 1e-3
		&& slot.min.z >= host.min.z - 1e-3
		&& slot.max.z <= host.max.z + 1e-3
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
	let along_x = along_is_x(region, node.abutment);
	let along = along_span(region, along_x);
	let n = ((along / 1.1).floor() as usize).clamp(1, 3);
	let mut out = Vec::new();
	for i in 0..n {
		let t = (i as f32 + 0.5) / n as f32;
		let c = if along_x {
			Vec3::new(region.min.x + along * t, region.min.y, (region.min.z + region.max.z) * 0.5)
		} else {
			Vec3::new((region.min.x + region.max.x) * 0.5, region.min.y, region.min.z + along * t)
		};
		let slot = Aabb3d::from_min_max(
			Vec3::new(c.x - CHAIR.x * 0.5, c.y, c.z - CHAIR.z * 0.5),
			Vec3::new(c.x + CHAIR.x * 0.5, c.y + CHAIR.y, c.z + CHAIR.z * 0.5),
		);
		let slot = clamp_xz(slot, region);
		if (slot.max.x - slot.min.x) < 0.3 || (slot.max.z - slot.min.z) < 0.3 {
			continue;
		}
		out.push(stamp_make(FurnitureNode::chair, &slot, region, node.abutment));
	}
	if out.is_empty() {
		let slab = floor_height_aabb(region, CHAIR.y);
		let c = (slab.min + slab.max) * 0.5;
		let slot = Aabb3d::from_min_max(
			Vec3::new(c.x - CHAIR.x * 0.5, slab.min.y, c.z - CHAIR.z * 0.5),
			Vec3::new(c.x + CHAIR.x * 0.5, slab.min.y + CHAIR.y, c.z + CHAIR.z * 0.5),
		);
		out.push(stamp_make(FurnitureNode::chair, &clamp_xz(slot, region), region, node.abutment));
	}
	out
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
		let tables = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Table).count();
		let chairs = pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count();
		assert!((1..=MAX_TABLES).contains(&tables), "expected tables, got {tables}");
		assert!(chairs >= 2, "expected chairs around tables, got {chairs}");
	}

	#[test]
	fn shallow_door_face_pocket_still_gets_a_table() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 3.2, 1.7));
		let pieces = BitesSeatingUsage::expand(&node);
		assert!(
			pieces.iter().any(|n| n.geometry == FurnitureGeometry::Table)
				|| pieces.iter().filter(|n| n.geometry == FurnitureGeometry::Chair).count() >= 2,
			"shallow seating should still allocate, got {:?}",
			pieces.iter().map(|n| n.geometry).collect::<Vec<_>>()
		);
	}

	#[test]
	fn tight_pocket_still_gets_a_chair() {
		let node = seating_node(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.3, 3.0, 1.3));
		let pieces = BitesSeatingUsage::expand(&node);
		assert!(pieces.iter().any(|n| n.geometry == FurnitureGeometry::Chair));
	}
}
