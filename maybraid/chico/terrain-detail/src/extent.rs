//! World-aligned formation (400 m) and outcropping (40 m) lattices.
//!
//! Half-open XZ, center-in-tile ownership — same rule as grove tiles
//! ([RFC-170 §3.1.1], [#785](https://github.com/ramate-io/maybraid/issues/785)).

use bevy::math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::Id;

/// Square formation tile span in metres on X and Z.
pub const DEFAULT_FORMATION_EXTENT_XZ: f32 = 400.0;

/// Square outcropping / planting cell span in metres on X and Z.
pub const DEFAULT_OUTCROPPING_EXTENT_XZ: f32 = 40.0;

/// Axis-aligned 400 m formation tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormationExtent {
	min: Vec3,
	max: Vec3,
}

impl FormationExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
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

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		Some(Self::new(bounds.min.into(), bounds.max.into()))
	}

	/// World-aligned tile containing `position` on the 400 m lattice.
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let s = DEFAULT_FORMATION_EXTENT_XZ;
		Self::new(
			Vec3::new(ix as f32 * s, 0.0, iz as f32 * s),
			Vec3::new((ix + 1) as f32 * s, 1.0, (iz + 1) as f32 * s),
		)
	}

	/// Cell index containing `position` on the origin-aligned 400 m grid.
	///
	/// The +X / +Z faces are exclusive so a point on a shared edge belongs to
	/// the higher-index neighbor.
	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let s = DEFAULT_FORMATION_EXTENT_XZ;
		((position.x / s).floor() as i32, (position.z / s).floor() as i32)
	}

	pub fn from_position(position: Vec3) -> Self {
		let (ix, iz) = Self::cell_index_containing(position);
		Self::from_cell_index(ix, iz)
	}

	/// Formation tiles whose footprints overlap `region` on XZ.
	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let s = DEFAULT_FORMATION_EXTENT_XZ;
		let ix0 = (region.min.x / s).floor() as i32;
		let iz0 = (region.min.z / s).floor() as i32;
		let max_x = (region.max.x - 1e-3).max(region.min.x);
		let max_z = (region.max.z - 1e-3).max(region.min.z);
		let ix1 = (max_x / s).floor() as i32;
		let iz1 = (max_z / s).floor() as i32;
		let (x0, x1) = (ix0.min(ix1), ix0.max(ix1));
		let (z0, z1) = (iz0.min(iz1), iz0.max(iz1));
		(x0..=x1)
			.flat_map(|ix| (z0..=z1).map(move |iz| Self::from_cell_index(ix, iz)))
			.collect()
	}

	/// Axis-aligned XZ disk of `radius` metres around `center`.
	pub fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
		let r = radius.max(0.0);
		Aabb3d::from_min_max(
			Vec3::new(center.x - r, 0.0, center.z - r),
			Vec3::new(center.x + r, 1.0, center.z + r),
		)
	}

	/// Half-open XZ (`[min, max)`) for a cell-center or placement.
	pub fn owns_center_xz(self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
	}

	/// World-aligned 40 m cells whose **center** lies in this formation tile.
	pub fn outcropping_cells(self) -> Vec<OutcroppingExtent> {
		OutcroppingExtent::cells_owned_by(self)
	}
}

/// Axis-aligned 40 m outcropping / planting cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutcroppingExtent {
	min: Vec3,
	max: Vec3,
}

impl OutcroppingExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
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

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		let size = bounds.max.x - bounds.min.x;
		if (size - DEFAULT_OUTCROPPING_EXTENT_XZ).abs() > 1e-2 {
			return None;
		}
		Some(Self::new(bounds.min.into(), bounds.max.into()))
	}

	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let s = DEFAULT_OUTCROPPING_EXTENT_XZ;
		Self::new(
			Vec3::new(ix as f32 * s, 0.0, iz as f32 * s),
			Vec3::new((ix + 1) as f32 * s, 1.0, (iz + 1) as f32 * s),
		)
	}

	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let s = DEFAULT_OUTCROPPING_EXTENT_XZ;
		((position.x / s).floor() as i32, (position.z / s).floor() as i32)
	}

	pub fn from_position(position: Vec3) -> Self {
		let (ix, iz) = Self::cell_index_containing(position);
		Self::from_cell_index(ix, iz)
	}

	/// Half-open XZ ownership for planting-cell centers (`[min, max)`).
	pub fn owns_center_xz(self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
	}

	/// World-aligned 40 m cells whose center lies in `formation`.
	pub fn cells_owned_by(formation: FormationExtent) -> Vec<Self> {
		let s = DEFAULT_OUTCROPPING_EXTENT_XZ;
		let min = formation.min();
		let max = formation.max();
		let ix0 = ((min.x / s).floor() as i32).saturating_sub(1);
		let ix1 = ((max.x / s).ceil() as i32).saturating_add(1);
		let iz0 = ((min.z / s).floor() as i32).saturating_sub(1);
		let iz1 = ((max.z / s).ceil() as i32).saturating_add(1);
		let mut cells = Vec::new();
		for ix in ix0..ix1 {
			for iz in iz0..iz1 {
				let cell = Self::from_cell_index(ix, iz);
				if formation.owns_center_xz(cell.center()) {
					cells.push(cell);
				}
			}
		}
		cells
	}

	/// 40 m cells whose footprints overlap `region` on XZ.
	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let s = DEFAULT_OUTCROPPING_EXTENT_XZ;
		let ix0 = (region.min.x / s).floor() as i32;
		let iz0 = (region.min.z / s).floor() as i32;
		let max_x = (region.max.x - 1e-3).max(region.min.x);
		let max_z = (region.max.z - 1e-3).max(region.min.z);
		let ix1 = (max_x / s).floor() as i32;
		let iz1 = (max_z / s).floor() as i32;
		let (x0, x1) = (ix0.min(ix1), ix0.max(ix1));
		let (z0, z1) = (iz0.min(iz1), iz0.max(iz1));
		(x0..=x1)
			.flat_map(|ix| (z0..=z1).map(move |iz| Self::from_cell_index(ix, iz)))
			.collect()
	}

	pub fn parent_formation(self) -> FormationExtent {
		FormationExtent::from_position(self.center())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn formation_index_is_half_open() -> Result<()> {
		assert_eq!(FormationExtent::cell_index_containing(Vec3::ZERO), (0, 0));
		assert_eq!(FormationExtent::cell_index_containing(Vec3::new(399.9, 0.0, 0.0)), (0, 0));
		assert_eq!(FormationExtent::cell_index_containing(Vec3::new(400.0, 0.0, 0.0)), (1, 0));
		assert_eq!(FormationExtent::cell_index_containing(Vec3::new(-0.1, 0.0, 0.0)), (-1, 0));
		Ok(())
	}

	#[test]
	fn owns_center_xz_excludes_max_face() -> Result<()> {
		let tile = FormationExtent::from_cell_index(0, 0);
		assert!(tile.owns_center_xz(Vec3::ZERO));
		assert!(tile.owns_center_xz(Vec3::new(0.0, 0.0, 0.0)));
		assert!(!tile.owns_center_xz(Vec3::new(400.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn formation_owns_ten_by_ten_outcropping_cells() -> Result<()> {
		let tile = FormationExtent::from_cell_index(0, 0);
		let cells = tile.outcropping_cells();
		assert_eq!(cells.len(), 10 * 10);
		assert!((cells[0].max().x - cells[0].min().x - DEFAULT_OUTCROPPING_EXTENT_XZ).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn adjacent_formations_do_not_share_a_cell_center() -> Result<()> {
		let west = FormationExtent::from_cell_index(0, 0);
		let east = FormationExtent::from_cell_index(1, 0);
		let west_mins: std::collections::HashSet<_> = west
			.outcropping_cells()
			.iter()
			.map(|c| (c.min().x.to_bits(), c.min().z.to_bits()))
			.collect();
		for cell in east.outcropping_cells() {
			let key = (cell.min().x.to_bits(), cell.min().z.to_bits());
			assert!(!west_mins.contains(&key), "shared planting cell at {key:?}");
		}
		Ok(())
	}

	#[test]
	fn outcropping_id_round_trips() -> Result<()> {
		let extent = OutcroppingExtent::from_cell_index(3, -2);
		let decoded = OutcroppingExtent::from_id(extent.id())
			.ok_or_else(|| anyhow::anyhow!("outcropping id"))?;
		assert_eq!(decoded, extent);
		Ok(())
	}
}
