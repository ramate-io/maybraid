//! Per-part UV atlas packing so DCC tools (Procreate) see non-overlapping islands in 0..1.

use bevy::math::{Vec2, Vec3};

/// Gutter as a fraction of each atlas tile, so packed islands do not touch.
const TILE_PAD: f32 = 0.06;
const EPS: f32 = 1e-5;

/// Axis-aligned UV island for one part / object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvIsland {
	pub min: Vec2,
	pub max: Vec2,
}

impl UvIsland {
	pub fn from_uvs(uvs: &[Vec2]) -> Option<Self> {
		if uvs.is_empty() {
			return None;
		}
		let mut min = Vec2::splat(f32::MAX);
		let mut max = Vec2::splat(f32::MIN);
		for uv in uvs {
			min = min.min(*uv);
			max = max.max(*uv);
		}
		Some(Self { min, max })
	}

	/// Strict interior overlap. Edge-touch (shared tile border) is not overlap.
	pub fn overlaps(self, other: Self) -> bool {
		self.min.x < other.max.x - EPS
			&& self.max.x > other.min.x + EPS
			&& self.min.y < other.max.y - EPS
			&& self.max.y > other.min.y + EPS
	}

	pub fn in_unit_square(self) -> bool {
		self.min.x >= -EPS
			&& self.min.y >= -EPS
			&& self.max.x <= 1.0 + EPS
			&& self.max.y <= 1.0 + EPS
	}
}

pub fn islands_overlap(islands: &[UvIsland]) -> bool {
	for i in 0..islands.len() {
		for j in (i + 1)..islands.len() {
			if islands[i].overlaps(islands[j]) {
				return true;
			}
		}
	}
	false
}

/// Project onto the two largest AABB axes (used when a part has no `vt`).
pub fn planar_unwrap(positions: &[Vec3]) -> Vec<Vec2> {
	if positions.is_empty() {
		return Vec::new();
	}
	let mut min = positions[0];
	let mut max = positions[0];
	for p in positions.iter().skip(1) {
		min = min.min(*p);
		max = max.max(*p);
	}
	let extent = max - min;
	// Drop the smallest axis so the projection keeps the most surface area.
	let (ax, ay) = if extent.x <= extent.y && extent.x <= extent.z {
		(1usize, 2usize)
	} else if extent.y <= extent.x && extent.y <= extent.z {
		(0, 2)
	} else {
		(0, 1)
	};
	positions
		.iter()
		.map(|p| {
			let coords = [p.x, p.y, p.z];
			Vec2::new(coords[ax], coords[ay])
		})
		.collect()
}

/// Stretch an island into 0..1. Degenerate islands sit at the tile center.
pub fn normalize_uvs(uvs: &[Vec2]) -> Vec<Vec2> {
	let Some(island) = UvIsland::from_uvs(uvs) else {
		return Vec::new();
	};
	let size = island.max - island.min;
	uvs.iter()
		.map(|uv| {
			let u = if size.x > EPS { (uv.x - island.min.x) / size.x } else { 0.5 };
			let v = if size.y > EPS { (uv.y - island.min.y) / size.y } else { 0.5 };
			Vec2::new(u, v)
		})
		.collect()
}

/// Existing UVs if they match vertex count; otherwise a planar unwrap.
pub fn source_uvs(positions: &[Vec3], uvs: &[Vec2]) -> Vec<Vec2> {
	if !uvs.is_empty() && uvs.len() == positions.len() {
		uvs.to_vec()
	} else {
		planar_unwrap(positions)
	}
}

/// Pack each already-sourced island into its own padded tile of the unit square.
pub fn pack_normalized_tiles(islands: &mut [Vec<Vec2>]) {
	let n = islands.len();
	if n == 0 {
		return;
	}
	let cols = (n as f32).sqrt().ceil() as usize;
	let rows = n.div_ceil(cols);
	let tile_w = 1.0 / cols as f32;
	let tile_h = 1.0 / rows as f32;
	let pad_x = tile_w * TILE_PAD;
	let pad_y = tile_h * TILE_PAD;
	let inner_w = (tile_w - 2.0 * pad_x).max(EPS);
	let inner_h = (tile_h - 2.0 * pad_y).max(EPS);

	for (i, uvs) in islands.iter_mut().enumerate() {
		let unit = normalize_uvs(uvs);
		let col = i % cols;
		let row = i / cols;
		let ox = col as f32 * tile_w + pad_x;
		let oy = row as f32 * tile_h + pad_y;
		*uvs = unit
			.iter()
			.map(|uv| Vec2::new(ox + uv.x * inner_w, oy + uv.y * inner_h))
			.collect();
	}
}

/// Unwrap missing UVs, then lay every part in a distinct 0..1 atlas tile.
pub fn pack_part_uvs(parts: &mut [(Vec<Vec3>, Vec<Vec2>)]) {
	let mut islands: Vec<Vec<Vec2>> =
		parts.iter().map(|(positions, uvs)| source_uvs(positions, uvs)).collect();
	pack_normalized_tiles(&mut islands);
	for ((_, dest), packed) in parts.iter_mut().zip(islands) {
		*dest = packed;
	}
}

pub fn islands_from_uvs(uvs: &[Vec<Vec2>]) -> Vec<UvIsland> {
	uvs.iter().filter_map(|u| UvIsland::from_uvs(u)).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn quad(z: f32) -> Vec<Vec3> {
		vec![
			Vec3::new(0.0, 0.0, z),
			Vec3::new(1.0, 0.0, z),
			Vec3::new(1.0, 1.0, z),
			Vec3::new(0.0, 1.0, z),
		]
	}

	fn unit_square_uvs() -> Vec<Vec2> {
		vec![Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
	}

	#[test]
	fn overlapping_unit_squares_are_detected() {
		let a = UvIsland::from_uvs(&unit_square_uvs()).unwrap();
		let b = UvIsland::from_uvs(&unit_square_uvs()).unwrap();
		assert!(a.overlaps(b));
		assert!(islands_overlap(&[a, b]));
	}

	#[test]
	fn mirrored_left_right_share_uvs_until_packed() {
		// Authoring often mirrors L/R eyes/ears onto the same 0..1 island.
		let mut parts = vec![
			(quad(0.0), unit_square_uvs()),
			(quad(0.0), unit_square_uvs()),
			(quad(1.0), Vec::new()),
		];
		let before: Vec<Vec<Vec2>> = parts.iter().map(|(p, u)| source_uvs(p, u)).collect();
		assert!(
			islands_overlap(&islands_from_uvs(&before)),
			"mirrored parts must overlap before atlas packing"
		);

		pack_part_uvs(&mut parts);
		let after: Vec<Vec<Vec2>> = parts.iter().map(|(_, u)| u.clone()).collect();
		let islands = islands_from_uvs(&after);
		assert_eq!(islands.len(), 3);
		assert!(!islands_overlap(&islands), "{islands:?}");
		for island in &islands {
			assert!(island.in_unit_square(), "{island:?}");
		}
		for (_, uvs) in &parts {
			assert_eq!(uvs.len(), 4);
			for uv in uvs {
				assert!((0.0..=1.0).contains(&uv.x), "{uv:?}");
				assert!((0.0..=1.0).contains(&uv.y), "{uv:?}");
			}
		}
	}

	#[test]
	fn unwraps_body_without_uvs_and_fits_unit_square() {
		let positions = quad(0.0);
		let uvs = planar_unwrap(&positions);
		assert_eq!(uvs.len(), positions.len());
		let packed = {
			let mut islands = vec![uvs];
			pack_normalized_tiles(&mut islands);
			islands.remove(0)
		};
		let island = UvIsland::from_uvs(&packed).unwrap();
		assert!(island.in_unit_square(), "{island:?}");
	}
}
