//! Shared ~2 m square shelf towers (0.75 m decks) and leftover stock/lounge fill.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::FurnitureNode;

use crate::bites_seating::BitesSeatingUsage;
use crate::region::{along_is_x, along_span, slice_along, stamp_make};

pub const SHELF_PLAN: f32 = 2.0;
pub const SHELF_DECK: f32 = 0.75;
pub const SHELF_DECKS: usize = 5;
pub const SHELF_GAP: f32 = 1.15;

/// Tile floor-standing shelf stacks in `floor`, skipping `keepouts` on XZ.
pub fn shelf_towers(
	floor: &Aabb3d,
	host: &Aabb3d,
	max_x: usize,
	max_z: usize,
	keepouts: &[Aabb3d],
) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx < SHELF_PLAN + 0.3 || sz < SHELF_PLAN + 0.3 {
		return Vec::new();
	}
	let n_x = count_units(sx, max_x);
	let n_z = count_units(sz, max_z);
	if n_x == 0 || n_z == 0 {
		return Vec::new();
	}
	let pitch = SHELF_PLAN + SHELF_GAP;
	let used_x = n_x as f32 * SHELF_PLAN + n_x.saturating_sub(1) as f32 * SHELF_GAP;
	let used_z = n_z as f32 * SHELF_PLAN + n_z.saturating_sub(1) as f32 * SHELF_GAP;
	let x0 = floor.min.x + ((sx - used_x) * 0.5).max(0.12);
	let z0 = floor.min.z + ((sz - used_z) * 0.5).max(0.12);
	let decks = ((floor.max.y - floor.min.y - 0.15) / SHELF_DECK).floor() as usize;
	let decks = decks.clamp(2, SHELF_DECKS);
	let mut out = Vec::new();
	for iz in 0..n_z {
		for ix in 0..n_x {
			let x = x0 + ix as f32 * pitch;
			let z = z0 + iz as f32 * pitch;
			let plan = Aabb3d::from_min_max(
				Vec3::new(x, floor.min.y, z),
				Vec3::new(x + SHELF_PLAN, floor.max.y, z + SHELF_PLAN),
			);
			if keepouts.iter().any(|k| overlaps_xz(&plan, k)) {
				continue;
			}
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

/// Fill a large leftover: lounge on one side, shelf towers on the other.
pub fn leftover_fill(floor: &Aabb3d, host: &Aabb3d, keepouts: &[Aabb3d]) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	let area = sx * sz;
	if area >= 16.0 && sx.min(sz) >= 2.8 {
		let along_x = along_is_x(floor, None);
		let along = along_span(floor, along_x);
		let lounge = slice_along(floor, along_x, 0.0, along * 0.40);
		let stock = slice_along(floor, along_x, along * 0.44, along);
		let mut out = BitesSeatingUsage::expand_aabb(&lounge, host);
		out.extend(shelf_towers(&stock, host, 3, 3, keepouts));
		out
	} else if sx.min(sz) >= SHELF_PLAN + 0.3 {
		shelf_towers(floor, host, 3, 3, keepouts)
	} else if sx.min(sz) >= 1.6 {
		BitesSeatingUsage::expand_aabb(floor, host)
	} else {
		Vec::new()
	}
}

pub fn overlaps_xz(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

fn count_units(span: f32, max: usize) -> usize {
	if span < SHELF_PLAN + 0.3 {
		return 0;
	}
	let n = ((span + SHELF_GAP) / (SHELF_PLAN + SHELF_GAP)).floor() as usize;
	n.clamp(1, max)
}
