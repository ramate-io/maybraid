//! Cardinal blend at forest-cell faces.
//!
//! A presenting 100 m tile still walks **its** planting cells. Near a face that
//! meets another forest cell, a hashed winner may run the neighbor recipe on
//! that cell instead. Interior tiles (same kind on all sides) never blend.

use bevy_math::Vec3;
use chico_groves::GroveExtent;

use crate::ForestGroveKind;

/// North, east, south, west.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinal {
	North,
	East,
	South,
	West,
}

impl Cardinal {
	pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

	pub fn index(self) -> usize {
		match self {
			Self::North => 0,
			Self::East => 1,
			Self::South => 2,
			Self::West => 3,
		}
	}
}

/// Blend-strip widths by forest layer (metres into the presenting tile).
pub const TUFT_BLEND_WIDTH: f32 = 12.0;
pub const UNDERSTORY_BLEND_WIDTH: f32 = 20.0;
pub const CANOPY_BLEND_WIDTH: f32 = 40.0;

/// Neighbor kind on each cardinal face. `None` = that face is not a forest-cell edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceNeighbors {
	pub north: Option<Option<ForestGroveKind>>,
	pub east: Option<Option<ForestGroveKind>>,
	pub south: Option<Option<ForestGroveKind>>,
	pub west: Option<Option<ForestGroveKind>>,
}

impl FaceNeighbors {
	pub fn none() -> Self {
		Self { north: None, east: None, south: None, west: None }
	}

	pub fn get(self, face: Cardinal) -> Option<Option<ForestGroveKind>> {
		match face {
			Cardinal::North => self.north,
			Cardinal::East => self.east,
			Cardinal::South => self.south,
			Cardinal::West => self.west,
		}
	}

	/// True when any open face has a kind different from `self_kind` (including empty).
	pub fn needs_blend(self, self_kind: ForestGroveKind) -> bool {
		Cardinal::ALL.iter().any(|&face| match self.get(face) {
			Some(Some(kind)) => kind != self_kind,
			Some(None) => true,
			None => false,
		})
	}
}

/// Weight in `0..1` that rises toward an open face (`1` on the edge, `0` past `width`).
pub fn face_weight(dist_to_face: f32, width: f32) -> f32 {
	let width = width.max(1e-3);
	(1.0 - (dist_to_face / width).clamp(0.0, 1.0)).max(0.0)
}

/// Hashed winner: `None` is this tile's recipe; `Some(face)` is that neighbor.
///
/// Only faces in `open` participate. At a corner the stronger face wins the
/// threshold; the hash still decides self vs neighbor.
pub fn pick_source(
	position: Vec3,
	tile: GroveExtent,
	width: f32,
	open: FaceNeighbors,
	hash_unit: f32,
) -> Option<Cardinal> {
	let hash = hash_unit.clamp(0.0, 1.0);
	let mut best: Option<(Cardinal, f32)> = None;
	for face in Cardinal::ALL {
		if open.get(face).is_none() {
			continue;
		}
		let dist = match face {
			Cardinal::East => tile.max().x - position.x,
			Cardinal::West => position.x - tile.min().x,
			Cardinal::North => tile.max().z - position.z,
			Cardinal::South => position.z - tile.min().z,
		};
		let w = face_weight(dist, width);
		if w > best.map(|(_, bw)| bw).unwrap_or(0.0) {
			best = Some((face, w));
		}
	}
	let Some((face, w)) = best else {
		return None;
	};
	if w > 0.0 && hash < w {
		Some(face)
	} else {
		None
	}
}

/// Stable `0..1` from a world XZ (cell center).
pub fn hash_unit_xz(position: Vec3) -> f32 {
	let h = position
		.x
		.to_bits()
		.wrapping_mul(0x9e3779b9)
		.wrapping_add(position.z.to_bits().wrapping_mul(0x85ebca77));
	(h as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;
	use chico_groves::GroveExtent;

	fn tile() -> GroveExtent {
		GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0))
	}

	fn east_open() -> FaceNeighbors {
		FaceNeighbors { east: Some(Some(ForestGroveKind::RollingOaks)), ..FaceNeighbors::none() }
	}

	#[test]
	fn interior_stays_self() -> Result<()> {
		let p = Vec3::new(50.0, 0.0, 50.0);
		assert_eq!(pick_source(p, tile(), 12.0, east_open(), 0.0), None);
		Ok(())
	}

	#[test]
	fn east_edge_can_pick_neighbor() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		assert_eq!(pick_source(p, tile(), 12.0, east_open(), 0.0), Some(Cardinal::East));
		assert_eq!(pick_source(p, tile(), 12.0, east_open(), 0.99), None);
		Ok(())
	}

	#[test]
	fn closed_face_never_blends() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		assert_eq!(pick_source(p, tile(), 12.0, FaceNeighbors::none(), 0.0), None);
		Ok(())
	}

	#[test]
	fn needs_blend_when_kind_differs() -> Result<()> {
		assert!(!east_open().needs_blend(ForestGroveKind::RollingOaks));
		assert!(east_open().needs_blend(ForestGroveKind::Orchard));
		Ok(())
	}
}
