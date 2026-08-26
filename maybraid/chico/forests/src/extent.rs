//! Forest cell footprint and grove-tile grid ([RFC-183 §3.5]).
//!
//! RFC starts at 1600 m forest cells with 200 m grove tiles. Production grove tiles
//! are [`chico_groves::DEFAULT_GROVE_EXTENT_XZ`] (100 m).

use bevy_math::Vec3;
use chico_groves::{GroveExtent, DEFAULT_GROVE_EXTENT_XZ};

/// Default square forest cell span in metres on X and Z.
pub const DEFAULT_FOREST_EXTENT_XZ: f32 = 1600.0;

/// Grove tile span used when a forest instantiates selected groves.
pub const DEFAULT_FOREST_GROVE_TILE_XZ: f32 = DEFAULT_GROVE_EXTENT_XZ;

/// Axis-aligned forest cell in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForestExtent {
	min: Vec3,
	max: Vec3,
}

impl ForestExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
	}

	/// Square cell of [`DEFAULT_FOREST_EXTENT_XZ`] centered on the origin.
	pub fn default_cell() -> Self {
		let h = DEFAULT_FOREST_EXTENT_XZ * 0.5;
		Self::new(Vec3::new(-h, 0.0, -h), Vec3::new(h, 1.0, h))
	}

	pub fn min(self) -> Vec3 {
		self.min
	}

	pub fn max(self) -> Vec3 {
		self.max
	}

	pub fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	/// Tile this forest cell into grove footprints of `tile_xz` metres.
	pub fn grove_tiles(self, tile_xz: f32) -> Vec<GroveExtent> {
		let tile = tile_xz.max(1.0);
		let span = self.max - self.min;
		let x_count = (span.x / tile).ceil().max(1.0) as u32;
		let z_count = (span.z / tile).ceil().max(1.0) as u32;
		let mut tiles = Vec::with_capacity((x_count * z_count) as usize);
		for x in 0..x_count {
			for z in 0..z_count {
				let min = Vec3::new(
					self.min.x + x as f32 * tile,
					self.min.y,
					self.min.z + z as f32 * tile,
				);
				let max = Vec3::new(
					(min.x + tile).min(self.max.x),
					self.max.y,
					(min.z + tile).min(self.max.z),
				);
				tiles.push(GroveExtent::new(min, max));
			}
		}
		tiles
	}

	pub fn default_grove_tiles(self) -> Vec<GroveExtent> {
		self.grove_tiles(DEFAULT_FOREST_GROVE_TILE_XZ)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_cell_is_sixteen_by_sixteen_hundred_metre_tiles() -> Result<()> {
		let tiles = ForestExtent::default_cell().default_grove_tiles();
		assert_eq!(tiles.len(), 16 * 16);
		assert!((tiles[0].max().x - tiles[0].min().x - DEFAULT_FOREST_GROVE_TILE_XZ).abs() < 1e-4);
		Ok(())
	}
}
