//! [`GenerationScheme`] for [`ChicoForest`] and forest-lattice region producers.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, OriginalId};
use lod::lod_ref::LodRef;
use lod::scene::{LodCullRegions, LodCullRegionsStatus, OpenLattice};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

use crate::index::{forest_world_sample, ForestIndex};
use crate::{assemble, select_cell, ChicoForest, ForestExtent, DEFAULT_FOREST_EXTENT_XZ};

impl GenerationScheme<ForestIndex> for ChicoForest {
	fn original_ids_for(_spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		ForestExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = ForestExtent::from_id(id)?;
		let layers = match spatial_index.layering {
			Some(kind) => kind.layering().typical_layers(),
			None => select_cell(extent, spatial_index.noise),
		};
		let neighbors = spatial_index.neighbor_layers(extent);
		let assembled = assemble(extent, layers, neighbors, &forest_world_sample());
		Some((Self { extent, assembled }, extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

/// Forest-cell bullseye: emit a Chebyshev ring AABB when the driver crosses a cell.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestGenerateBullseye {
	pub radius: u32,
	pub enabled: bool,
}

impl Default for ForestGenerateBullseye {
	fn default() -> Self {
		Self { radius: 2, enabled: false }
	}
}

impl LodRefreshRegions for ForestGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = ForestExtent::cell_index_containing(lod_ref.previous_transform.translation);
		let current = ForestExtent::cell_index_committed(
			lod_ref.current_transform.translation,
			previous,
			80.0,
		);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::ring_aabb(current, self.radius))
	}
}

/// Present ring — typically one cell smaller than [`ForestGenerateBullseye`].
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestPresentBullseye {
	pub radius: u32,
	pub enabled: bool,
}

impl Default for ForestPresentBullseye {
	fn default() -> Self {
		Self { radius: 1, enabled: false }
	}
}

impl LodRefreshRegions for ForestPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = ForestExtent::cell_index_containing(lod_ref.previous_transform.translation);
		let current = ForestExtent::cell_index_committed(
			lod_ref.current_transform.translation,
			previous,
			80.0,
		);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::ring_aabb(current, self.radius))
	}
}

/// Channel marker for forest generate / present messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct ForestLodChan;

/// Present-layer cull lattice (not the scene [`OpenLattice`] resource).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestPresentLattice {
	pub lattice: OpenLattice,
	pub enabled: bool,
}

impl Default for ForestPresentLattice {
	fn default() -> Self {
		Self { lattice: OpenLattice::new(4800.0, 9600.0, DEFAULT_FOREST_EXTENT_XZ), enabled: false }
	}
}

impl ForestPresentLattice {
	pub fn from_stream_radius(present_radius: u32) -> Self {
		let cell = DEFAULT_FOREST_EXTENT_XZ;
		let present_edge = cell * (2 * present_radius + 1) as f32;
		let generate_edge = cell * (2 * present_radius.saturating_add(1) + 1) as f32;
		Self { lattice: OpenLattice::new(present_edge, generate_edge + cell, cell), enabled: true }
	}
}

impl LodCullRegions for ForestPresentLattice {
	fn lod_cull_regions(
		&self,
		lod_refs: &[LodRef],
		cursor: &mut lod::scene::LodCullRegionCursor,
	) -> LodCullRegionsStatus {
		if !self.enabled || lod_refs.is_empty() {
			return LodCullRegionsStatus::Unchanged;
		}
		self.lattice.lod_cull_regions(lod_refs, cursor)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::GeneratingSpatialIndex;
	use lod::lod_ref::LodRef;

	#[test]
	fn original_ids_are_overlapping_forest_cells() -> Result<()> {
		let region = ForestExtent::ring_aabb((0, 0), 1);
		let ids = ChicoForest::original_ids_for(&mut ForestIndex::default(), region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}

	#[test]
	fn build_inserts_a_cell() -> Result<()> {
		let mut index = ForestIndex::default();
		let extent =
			ForestExtent::new(bevy::math::Vec3::ZERO, bevy::math::Vec3::new(100.0, 1.0, 100.0));
		let id = extent.id();
		let identity = bevy::prelude::Transform::IDENTITY;
		let bounds = extent.aabb();
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<ChicoForest>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		assert!(lod::gen::SpatialIndex::<ChicoForest>::get(&index, id).is_some());
		Ok(())
	}
}
