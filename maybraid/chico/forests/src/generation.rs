//! [`GenerationScheme`] for [`ChicoForest`] (dependency) and [`ChicoGrove`] (origins).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::scene::{LodCullRegions, LodCullRegionsStatus, OpenLattice};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

use crate::grove::{grove_from_id, grove_id};
use crate::index::ForestIndex;
use crate::{
	presenting_recipes, ChicoForest, ChicoGrove, ForestExtent, ForestLayer,
	DEFAULT_FOREST_GROVE_TILE_XZ,
};

/// Generate ring around the camera (metres). Present is closer; see [`GROVE_PRESENT_RADIUS_M`].
pub const GROVE_GENERATE_RADIUS_M: f32 = 2000.0;

/// Present ring around the camera (metres).
pub const GROVE_PRESENT_RADIUS_M: f32 = 1000.0;

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
		let layers = spatial_index.selected_layers_for(extent);
		Some((Self { extent, layers }, extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

impl GenerationScheme<ForestIndex> for ChicoGrove {
	fn original_ids_for(spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = Vec::new();
		for tile in ForestExtent::grove_tiles_overlapping(region) {
			let center = (tile.min() + tile.max()) * 0.5;
			let forest = ForestExtent::from_cell_index(
				ForestExtent::cell_index_containing(center).0,
				ForestExtent::cell_index_containing(center).1,
			);
			ensure_forest_ring(spatial_index, forest);
			let Some(selected) = SpatialIndex::<ChicoForest>::get(spatial_index, forest.id())
			else {
				continue;
			};
			let layers = selected.layers;
			let neighbors = spatial_index.neighbor_layers(forest);
			for layer in ForestLayer::ALL {
				if layer.kind(layers).is_some() || neighbors.any_kind(layer) {
					ids.push(OriginalId(grove_id(tile, layer)));
				}
			}
		}
		ids
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let (extent, layer) = grove_from_id(id)?;
		let center = (extent.min() + extent.max()) * 0.5;
		let forest_extent = ForestExtent::from_cell_index(
			ForestExtent::cell_index_containing(center).0,
			ForestExtent::cell_index_containing(center).1,
		);
		GeneratingSpatialIndex::<ChicoForest>::get_or_generate(
			spatial_index,
			forest_extent.id(),
			lod_ref,
		)?;
		ensure_forest_ring(spatial_index, forest_extent);
		let forest = SpatialIndex::<ChicoForest>::get(spatial_index, forest_extent.id())?;
		let kind = layer.kind(forest.layers);
		let neighbors = spatial_index.neighbor_layers(forest_extent);
		let recipes = presenting_recipes(kind, extent, forest_extent, &neighbors, layer);
		Some((
			Self::selected(extent, layer, recipes),
			grove_id(extent, layer).origin_cell_bounds()?,
		))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

fn ensure_forest_ring(index: &mut ForestIndex, forest: ForestExtent) {
	index.ensure_forest_selected(forest);
	let (ix, iz) = ForestExtent::cell_index_containing(forest.center());
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix, iz + 1));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix + 1, iz));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix, iz - 1));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix - 1, iz));
}

/// Grove-ring bullseye: emit a metric AABB when the driver crosses a 100 m tile.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for ForestGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: GROVE_GENERATE_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for ForestGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = grove_tile_index(lod_ref.previous_transform.translation);
		let current = grove_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

/// Present ring — typically 1 km when generate is 2 km.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for ForestPresentBullseye {
	fn default() -> Self {
		Self { radius_m: GROVE_PRESENT_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for ForestPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = grove_tile_index(lod_ref.previous_transform.translation);
		let current = grove_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

fn grove_tile_index(position: Vec3) -> (i32, i32) {
	let s = DEFAULT_FOREST_GROVE_TILE_XZ;
	let origin = -crate::DEFAULT_FOREST_EXTENT_XZ * 0.5;
	(((position.x - origin) / s).floor() as i32, ((position.z - origin) / s).floor() as i32)
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
		Self {
			lattice: OpenLattice::new(
				GROVE_PRESENT_RADIUS_M * 2.0,
				GROVE_GENERATE_RADIUS_M * 2.0 + DEFAULT_FOREST_GROVE_TILE_XZ,
				DEFAULT_FOREST_GROVE_TILE_XZ,
			),
			enabled: false,
		}
	}
}

impl ForestPresentLattice {
	pub fn from_radii_m(present_m: f32, generate_m: f32) -> Self {
		Self {
			lattice: OpenLattice::new(
				present_m.max(1.0) * 2.0,
				generate_m.max(present_m) * 2.0 + DEFAULT_FOREST_GROVE_TILE_XZ,
				DEFAULT_FOREST_GROVE_TILE_XZ,
			),
			enabled: true,
		}
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

	fn test_lod_ref(bounds: Aabb3d) -> (bevy::prelude::Transform, Aabb3d) {
		(bevy::prelude::Transform::IDENTITY, bounds)
	}

	#[test]
	fn forest_original_ids_are_overlapping_forest_cells() -> Result<()> {
		let region = ForestExtent::ring_aabb((0, 0), 1);
		let ids = ChicoForest::original_ids_for(&mut ForestIndex::default(), region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}

	#[test]
	fn forest_build_is_select_only() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let extent = ForestExtent::default_cell();
		let id = extent.id();
		let (identity, bounds) = test_lod_ref(extent.aabb());
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<ChicoForest>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		let forest = lod::gen::SpatialIndex::<ChicoForest>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("forest"))?;
		assert_eq!(forest.layers.layering, crate::LayeringKind::LushJungle);
		assert!(forest.layers.upper_canopy.is_some());
		Ok(())
	}

	#[test]
	fn grove_origins_are_one_id_per_selected_layer() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let region = ForestExtent::xz_radius_aabb(Vec3::new(50.0, 0.0, 50.0), 10.0);
		let tile = ForestExtent::grove_tiles_overlapping(region);
		assert_eq!(tile.len(), 1);
		let ids = ChicoGrove::original_ids_for(&mut index, region);
		let layers = crate::LayeringKind::LushJungle.layering().typical_layers();
		let expected = ForestLayer::ALL.iter().filter(|layer| layer.kind(layers).is_some()).count();
		assert_eq!(ids.len(), expected);
		assert!(expected > 1, "lush jungle should select more than one layer");
		Ok(())
	}

	#[test]
	fn grove_build_depends_on_forest_and_does_not_grow() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let region = ForestExtent::xz_radius_aabb(Vec3::ZERO, 50.0);
		let ids = ChicoGrove::original_ids_for(&mut index, region);
		let id = ids.first().ok_or_else(|| anyhow::anyhow!("grove id"))?.0;
		let (identity, bounds) = test_lod_ref(region);
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<ChicoGrove>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		let grove = lod::gen::SpatialIndex::<ChicoGrove>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("grove"))?;
		assert!(!grove.recipes.is_empty());
		assert!(grove.grown_tiles().is_none());
		let forest_id = ForestExtent::default_cell().id();
		assert!(lod::gen::SpatialIndex::<ChicoForest>::get(&index, forest_id).is_some());
		Ok(())
	}
}
