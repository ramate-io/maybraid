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

	/// Half-open XZ (`[min, max)`) for a grove-slot center.
	pub fn owns_center_xz(self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
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

	/// Origin-centered cell whose `(0, 0)` index is [`Self::default_cell`].
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let s = DEFAULT_FOREST_EXTENT_XZ;
		let h = s * 0.5;
		Self::new(
			Vec3::new(ix as f32 * s - h, 0.0, iz as f32 * s - h),
			Vec3::new(ix as f32 * s + h, 1.0, iz as f32 * s + h),
		)
	}

	/// Cell index containing `position` on the origin-centered 1600 m grid.
	///
	/// The +X / +Z faces are exclusive so a point on a shared edge belongs to the
	/// higher-index neighbor.
	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let s = DEFAULT_FOREST_EXTENT_XZ;
		let h = s * 0.5;
		let ix = ((position.x + h) / s).floor() as i32;
		let iz = ((position.z + h) / s).floor() as i32;
		(ix, iz)
	}

	/// Inclusive Chebyshev ring of cell indices around `center`.
	pub fn cell_ring(center: (i32, i32), radius: u32) -> impl Iterator<Item = (i32, i32)> {
		let r = radius as i32;
		let (cx, cz) = center;
		(-r..=r).flat_map(move |dx| (-r..=r).map(move |dz| (cx + dx, cz + dz)))
	}

	/// Stay on `current` until `position` is `margin` metres inside a neighboring cell.
	///
	/// Stops the streamer from thrashing when the camera sits on a shared face.
	pub fn cell_index_committed(position: Vec3, current: (i32, i32), margin: f32) -> (i32, i32) {
		let raw = Self::cell_index_containing(position);
		if raw == current {
			return current;
		}
		let next = Self::from_cell_index(raw.0, raw.1);
		let m = margin.max(0.0);
		let min = next.min();
		let max = next.max();
		let committed = position.x >= min.x + m
			&& position.x <= max.x - m
			&& position.z >= min.z + m
			&& position.z <= max.z - m;
		if committed {
			raw
		} else {
			current
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn owns_center_xz_is_half_open() -> Result<()> {
		let cell = ForestExtent::default_cell();
		assert!(cell.owns_center_xz(Vec3::ZERO));
		assert!(cell.owns_center_xz(Vec3::new(-800.0, 0.0, 0.0)));
		assert!(!cell.owns_center_xz(Vec3::new(800.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn default_cell_is_sixteen_by_sixteen_hundred_metre_tiles() -> Result<()> {
		let tiles = ForestExtent::default_cell().default_grove_tiles();
		assert_eq!(tiles.len(), 16 * 16);
		assert!((tiles[0].max().x - tiles[0].min().x - DEFAULT_FOREST_GROVE_TILE_XZ).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn origin_cell_index_matches_default_cell() -> Result<()> {
		assert_eq!(ForestExtent::from_cell_index(0, 0), ForestExtent::default_cell());
		assert_eq!(ForestExtent::cell_index_containing(Vec3::ZERO), (0, 0));
		assert_eq!(ForestExtent::cell_index_containing(Vec3::new(799.9, 0.0, 0.0)), (0, 0));
		assert_eq!(ForestExtent::cell_index_containing(Vec3::new(800.0, 0.0, 0.0)), (1, 0));
		assert_eq!(ForestExtent::cell_index_containing(Vec3::new(-800.0, 0.0, 0.0)), (0, 0));
		assert_eq!(ForestExtent::cell_index_containing(Vec3::new(-800.1, 0.0, 0.0)), (-1, 0));
		Ok(())
	}

	#[test]
	fn cell_ring_radius_one_is_three_by_three() -> Result<()> {
		let cells: Vec<_> = ForestExtent::cell_ring((0, 0), 1).collect();
		assert_eq!(cells.len(), 9);
		assert!(cells.contains(&(0, 0)));
		assert!(cells.contains(&(1, -1)));
		Ok(())
	}

	#[test]
	fn cell_index_committed_ignores_shared_face() -> Result<()> {
		let on_face = Vec3::new(800.0, 0.0, 0.0);
		assert_eq!(ForestExtent::cell_index_containing(on_face), (1, 0));
		assert_eq!(ForestExtent::cell_index_committed(on_face, (0, 0), 80.0), (0, 0));
		let inside_next = Vec3::new(880.0, 0.0, 0.0);
		assert_eq!(ForestExtent::cell_index_committed(inside_next, (0, 0), 80.0), (1, 0));
		Ok(())
	}
}
