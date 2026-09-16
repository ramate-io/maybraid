//! Shared ~2 m square shelf towers, floor-to-ceiling partitions, leftover fill.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use richmond_building_components::{FurnitureAbutment, FurnitureNode};

use crate::bites_seating::BitesSeatingUsage;
use crate::region::{along_is_x, along_span, slice_along, stamp_make};

pub const SHELF_PLAN: f32 = 2.0;
pub const SHELF_DECK: f32 = 0.75;
pub const SHELF_DECKS: usize = 5;
pub const SHELF_GAP: f32 = 1.15;

const PARTITION_LEN: f32 = 2.6;
const PARTITION_THICK: f32 = 0.22;
const PARTITION_GAP: f32 = 2.2;
const PARTITION_PITCH: f32 = PARTITION_LEN + PARTITION_GAP;

/// Kitchen leftovers stay work-like; hall leftovers may take a lounge pocket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftoverMood {
	Kitchen,
	Hall,
}

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

/// Fill a large leftover: lounge pocket, slat screens, then a stock corner.
pub fn leftover_fill(
	floor: &Aabb3d,
	host: &Aabb3d,
	keepouts: &[Aabb3d],
	mood: LeftoverMood,
) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	let area = sx * sz;
	let along_x = along_is_x(floor, None);
	let along = along_span(floor, along_x);
	let want_lounge = match mood {
		LeftoverMood::Kitchen => area >= 42.0 && sx.min(sz) >= 4.0,
		LeftoverMood::Hall => area >= 24.0 && sx.min(sz) >= 3.6,
	};
	let mut lounge_span = if want_lounge { 4.4 } else { 0.0 };
	let mut stock_span = if sx.min(sz) >= SHELF_PLAN + 0.3 { 3.3 } else { 0.0 };
	if lounge_span + stock_span + 2.2 > along {
		lounge_span = 0.0;
	}
	if stock_span + 2.2 > along {
		stock_span = 0.0;
	}

	let mut out = Vec::new();
	let mut cursor = 0.0;
	if lounge_span > 0.0 {
		let band = slice_along(floor, along_x, cursor, cursor + lounge_span);
		out.extend(BitesSeatingUsage::expand_aabb(&square_in(&band, 4.3), host));
		cursor += lounge_span + 0.25;
	}
	let hall_end = along - stock_span;
	if hall_end - cursor >= 2.0 {
		let hall = slice_along(floor, along_x, cursor, hall_end);
		out.extend(partition_bays(&hall, host, keepouts));
	}
	if stock_span > 0.0 {
		let stock = slice_along(floor, along_x, along - stock_span, along);
		out.extend(shelf_towers(&stock, host, 2, 2, keepouts));
	}
	if out.is_empty() {
		out.extend(shelf_towers(floor, host, 2, 2, keepouts));
		if out.is_empty() && sx.min(sz) >= 1.6 && mood == LeftoverMood::Hall {
			out.extend(BitesSeatingUsage::expand_aabb(floor, host));
		}
	}
	out
}

/// Floor-to-ceiling slat screens along the long axis of `floor`.
pub fn partition_bays(floor: &Aabb3d, host: &Aabb3d, keepouts: &[Aabb3d]) -> Vec<FurnitureNode> {
	let sx = floor.max.x - floor.min.x;
	let sz = floor.max.z - floor.min.z;
	if sx.max(sz) < 2.4 || sx.min(sz) < 1.2 {
		return Vec::new();
	}
	let along_x = sx + 1e-4 >= sz;
	let along = if along_x { sx } else { sz };
	let n = count_bays(along);
	if n == 0 {
		return Vec::new();
	}
	let used = n as f32 * PARTITION_LEN + n.saturating_sub(1) as f32 * PARTITION_GAP;
	let start = if along_x {
		floor.min.x + ((along - used) * 0.5).max(0.15)
	} else {
		floor.min.z + ((along - used) * 0.5).max(0.15)
	};
	let height = (floor.max.y - floor.min.y - 0.04).max(2.4);
	let side = if along_x { FurnitureAbutment::NegZ } else { FurnitureAbutment::NegX };
	let mut out = Vec::new();
	for i in 0..n {
		let t0 = start + i as f32 * PARTITION_PITCH;
		let slot = if along_x {
			let mid = (floor.min.z + floor.max.z) * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(t0, floor.min.y, mid - PARTITION_THICK * 0.5),
				Vec3::new(t0 + PARTITION_LEN, floor.min.y + height, mid + PARTITION_THICK * 0.5),
			)
		} else {
			let mid = (floor.min.x + floor.max.x) * 0.5;
			Aabb3d::from_min_max(
				Vec3::new(mid - PARTITION_THICK * 0.5, floor.min.y, t0),
				Vec3::new(mid + PARTITION_THICK * 0.5, floor.min.y + height, t0 + PARTITION_LEN),
			)
		};
		if keepouts.iter().any(|k| overlaps_xz(&slot, k)) {
			continue;
		}
		out.push(stamp_make(FurnitureNode::partition, &slot, host, Some(side)));
	}
	out
}

fn square_in(band: &Aabb3d, side: f32) -> Aabb3d {
	let sx = (band.max.x - band.min.x).min(side);
	let sz = (band.max.z - band.min.z).min(side);
	Aabb3d::from_min_max(
		Vec3::new(band.min.x + 0.12, band.min.y, band.min.z + 0.12),
		Vec3::new(band.min.x + 0.12 + sx, band.max.y, band.min.z + 0.12 + sz),
	)
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

fn count_bays(along: f32) -> usize {
	if along < PARTITION_LEN + 0.2 {
		return 0;
	}
	let n = ((along + PARTITION_GAP) / PARTITION_PITCH).floor() as usize;
	n.clamp(1, 4)
}
