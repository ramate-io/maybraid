//! Open lattice: emit rotating annulus tiles (center hole excluded).

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec3, Vec3};
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::cursor::LodCullRegionCursor;
use super::produce::{LodCullRegions, LodCullRegionsStatus};

/// Y half-extent of each emitted tile. Avian `hosts_in_region` is 3D; a cube
/// at camera altitude misses ground grove AABBs. This slab is an XZ query.
const CULL_SLAB_Y: f32 = 1_000_000.0;

/// Cull region producer: ground-plane lattice with a center hole.
///
/// Enumerates XZ tiles (Y locked to 0) whose centers lie inside the outer
/// square but outside the exclude square, then round-robins through them via
/// [`LodCullRegionCursor`]. Emitted AABBs are XZ cells with a tall Y slab so
/// a high fly still hits hosts on the ground.
///
/// Defaults match a large vegetation GC ring: 1 km hole, 5 km outer, 500 m tiles.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct OpenLattice {
	/// Edge length of the excluded center cube (world units).
	pub exclude_extent: f32,
	/// Edge length of the outer coverage cube (world units).
	pub outer_extent: f32,
	/// Lattice tile edge length (world units).
	pub tile_size: f32,
}

impl Default for OpenLattice {
	fn default() -> Self {
		Self { exclude_extent: 1000.0, outer_extent: 5000.0, tile_size: 500.0 }
	}
}

impl OpenLattice {
	pub fn new(exclude_extent: f32, outer_extent: f32, tile_size: f32) -> Self {
		Self { exclude_extent, outer_extent, tile_size }
	}

	fn tile(&self) -> f32 {
		self.tile_size.max(1e-3)
	}

	fn cell_index(&self, point: Vec3) -> IVec3 {
		let s = self.tile();
		IVec3::new((point.x / s).floor() as i32, 0, (point.z / s).floor() as i32)
	}

	fn cell_center(&self, index: IVec3) -> Vec3 {
		(index.as_vec3() + Vec3::splat(0.5)) * self.tile()
	}

	fn tile_aabb(&self, index: IVec3) -> Aabb3d {
		let s = self.tile();
		let min = Vec3::new(index.x as f32 * s, -CULL_SLAB_Y, index.z as f32 * s);
		let max = Vec3::new(min.x + s, CULL_SLAB_Y, min.z + s);
		Aabb3d::from_min_max(min, max)
	}

	fn in_cube(point: Vec3, center: Vec3, edge: f32) -> bool {
		let half = edge.max(0.0) * 0.5;
		let d = (point - center).abs();
		d.x <= half && d.y <= half && d.z <= half
	}

	/// XZ annulus cells around `anchor` (Y is 0).
	pub fn enumerate_cells(&self, anchor: IVec3) -> Vec<IVec3> {
		let tile = self.tile();
		let half_outer = ((self.outer_extent * 0.5) / tile).ceil() as i32;
		let anchor_center = self.cell_center(anchor);
		let mut cells = Vec::new();
		for dx in -half_outer..=half_outer {
			for dz in -half_outer..=half_outer {
				let cell = IVec3::new(anchor.x + dx, anchor.y, anchor.z + dz);
				let center = self.cell_center(cell);
				if !Self::in_cube(center, anchor_center, self.outer_extent) {
					continue;
				}
				if Self::in_cube(center, anchor_center, self.exclude_extent) {
					continue;
				}
				cells.push(cell);
			}
		}
		cells
	}
}

impl LodCullRegions for OpenLattice {
	fn lod_cull_regions(
		&self,
		lod_refs: &[LodRef],
		cursor: &mut LodCullRegionCursor,
	) -> LodCullRegionsStatus {
		let Some(driver) = lod_refs.first() else {
			return LodCullRegionsStatus::Unchanged;
		};
		let anchor = self.cell_index(driver.current_transform.translation);
		cursor.ensure_cells(anchor, || self.enumerate_cells(anchor));
		let batch = cursor.take_cells();
		if batch.is_empty() {
			return LodCullRegionsStatus::Unchanged;
		}
		LodCullRegionsStatus::Changed(batch.into_iter().map(|c| self.tile_aabb(c)).collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;

	fn lod_ref_at<'a>(prev: &'a Transform, curr: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::from_bits(1),
			previous_transform: prev,
			current_transform: curr,
			bounds,
		}
	}

	#[test]
	fn hole_and_outer_exclude_center() {
		let lattice = OpenLattice::new(1000.0, 5000.0, 500.0);
		let cells = lattice.enumerate_cells(IVec3::ZERO);
		assert!(!cells.is_empty());
		assert!(!cells.contains(&IVec3::ZERO));
		// Immediate neighbor at 500 m center offset is inside 1 km hole.
		assert!(!cells.contains(&IVec3::new(1, 0, 0)));
		// Farther ring should appear.
		assert!(cells.iter().any(|c| c.x.abs() >= 2 || c.z.abs() >= 2));
	}

	#[test]
	fn cursor_loops_and_rebuilds_on_anchor_change() {
		let lattice = OpenLattice::new(1000.0, 3000.0, 500.0);
		let mut cursor = LodCullRegionCursor::default().with_regions_per_tick(1);
		let t = Transform::from_translation(Vec3::ZERO);
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let r = lod_ref_at(&t, &t, &bounds);
		let status = lattice.lod_cull_regions(&[r], &mut cursor);
		let LodCullRegionsStatus::Changed(first) = status else {
			panic!("expected changed");
		};
		assert_eq!(first.len(), 1);
		let n = cursor.cells.len();
		assert!(n > 1);
		for _ in 0..n + 2 {
			let _ = lattice.lod_cull_regions(&[r], &mut cursor);
		}
		assert_eq!(cursor.anchor_cell, Some(IVec3::ZERO));

		let t2 = Transform::from_translation(Vec3::new(600.0, 0.0, 0.0));
		let r2 = lod_ref_at(&t, &t2, &bounds);
		let _ = lattice.lod_cull_regions(&[r2], &mut cursor);
		assert_ne!(cursor.anchor_cell, Some(IVec3::ZERO));
		assert_eq!(cursor.next, 1); // rebuilt then took one
	}

	#[test]
	fn same_anchor_keeps_cursor_without_rebuild() {
		let lattice = OpenLattice::new(1000.0, 3000.0, 500.0);
		let mut cursor = LodCullRegionCursor::default().with_regions_per_tick(1);
		let t = Transform::from_translation(Vec3::ZERO);
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let r = lod_ref_at(&t, &t, &bounds);
		let _ = lattice.lod_cull_regions(&[r], &mut cursor);
		let n = cursor.cells.len();
		assert!(n > 1);
		assert_eq!(cursor.next, 1);
		let _ = lattice.lod_cull_regions(&[r], &mut cursor);
		assert_eq!(cursor.anchor_cell, Some(IVec3::ZERO));
		assert_eq!(cursor.next, 2);
		assert_eq!(cursor.cells.len(), n);

		cursor.invalidate_cells();
		let _ = lattice.lod_cull_regions(&[r], &mut cursor);
		assert_eq!(cursor.anchor_cell, Some(IVec3::ZERO));
		assert_eq!(cursor.next, 1);
		assert_eq!(cursor.cells.len(), n);
	}

	fn aabb_overlap_xz_y(a: Aabb3d, b: Aabb3d) -> bool {
		a.min.x <= b.max.x
			&& a.max.x >= b.min.x
			&& a.min.y <= b.max.y
			&& a.max.y >= b.min.y
			&& a.min.z <= b.max.z
			&& a.max.z >= b.min.z
	}

	#[test]
	fn high_driver_tiles_overlap_ground_grove_aabb() {
		let lattice = OpenLattice::new(1000.0, 5000.0, 500.0);
		let mut cursor = LodCullRegionCursor::default().with_regions_per_tick(1);
		let t = Transform::from_translation(Vec3::new(0.0, 600.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let r = lod_ref_at(&t, &t, &bounds);
		let LodCullRegionsStatus::Changed(tiles) = lattice.lod_cull_regions(&[r], &mut cursor)
		else {
			panic!("expected changed");
		};
		assert_eq!(tiles.len(), 1);
		assert!(tiles[0].min.y <= 0.0 && tiles[0].max.y >= 1.0);
		let ground = Aabb3d::from_min_max(
			Vec3::new(tiles[0].min.x, 0.0, tiles[0].min.z),
			Vec3::new(tiles[0].max.x, 1.0, tiles[0].max.z),
		);
		assert!(aabb_overlap_xz_y(tiles[0], ground));
		assert_eq!(cursor.anchor_cell, Some(IVec3::ZERO));
	}

	#[test]
	fn high_driver_tile_hits_ground_host_800m_xz() {
		let lattice = OpenLattice::new(1000.0, 5000.0, 500.0);
		let cell = lattice.cell_index(Vec3::new(800.0, 600.0, 0.0));
		let tile = lattice.tile_aabb(cell);
		let host = Aabb3d::from_min_max(Vec3::new(790.0, 0.0, -10.0), Vec3::new(810.0, 8.0, 10.0));
		assert!(aabb_overlap_xz_y(tile, host));
	}

	#[test]
	fn flying_up_does_not_rebuild_xz_cursor() {
		let lattice = OpenLattice::new(1000.0, 3000.0, 500.0);
		let mut cursor = LodCullRegionCursor::default().with_regions_per_tick(1);
		let t0 = Transform::from_translation(Vec3::ZERO);
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let r0 = lod_ref_at(&t0, &t0, &bounds);
		let _ = lattice.lod_cull_regions(&[r0], &mut cursor);
		assert_eq!(cursor.next, 1);
		let t1 = Transform::from_translation(Vec3::new(0.0, 600.0, 0.0));
		let r1 = lod_ref_at(&t0, &t1, &bounds);
		let _ = lattice.lod_cull_regions(&[r1], &mut cursor);
		assert_eq!(cursor.anchor_cell, Some(IVec3::ZERO));
		assert_eq!(cursor.next, 2);
	}
}
