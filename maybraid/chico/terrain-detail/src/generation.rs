//! [`GenerationScheme`] for [`TerrainDetail`] (dependency) and
//! [`TerrainOutcropping`] (origins that grow).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

use crate::{
	FormationExtent, OutcroppingExtent, TerrainDetail, TerrainDetailIndex, TerrainOutcropping,
};

/// Formation generate ring around the camera (metres).
pub const TERRAIN_DETAIL_GENERATE_RADIUS_M: f32 = 1600.0;

/// Outcropping present ring around the camera (metres).
pub const TERRAIN_DETAIL_PRESENT_RADIUS_M: f32 = 800.0;

impl GenerationScheme<TerrainDetailIndex> for TerrainDetail {
	fn original_ids_for(
		_spatial_index: &mut TerrainDetailIndex,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		FormationExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut TerrainDetailIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = FormationExtent::from_id(id)?;
		let formation = spatial_index.selected_formation_for(extent);
		Some((Self { extent, formation }, extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut TerrainDetailIndex,
		_lod_ref: &LodRef,
	) {
	}
}

impl GenerationScheme<TerrainDetailIndex> for TerrainOutcropping {
	fn original_ids_for(spatial_index: &mut TerrainDetailIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = Vec::new();
		for formation in FormationExtent::cells_overlapping(region) {
			spatial_index.ensure_detail_selected(formation);
			let Some(detail) = SpatialIndex::<TerrainDetail>::get(spatial_index, formation.id())
			else {
				continue;
			};
			let kind = detail.formation;
			for cell in formation.outcropping_cells() {
				if !region_overlaps_cell(region, cell) {
					continue;
				}
				if kind.throw_outcropping(cell, spatial_index.noise).is_some() {
					ids.push(OriginalId(cell.id()));
				}
			}
		}
		ids
	}

	fn build_with_id(
		spatial_index: &mut TerrainDetailIndex,
		id: lod::gen::Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = OutcroppingExtent::from_id(id)?;
		let parent = extent.parent_formation();
		GeneratingSpatialIndex::<TerrainDetail>::get_or_generate(
			spatial_index,
			parent.id(),
			lod_ref,
		)?;
		let detail = SpatialIndex::<TerrainDetail>::get(spatial_index, parent.id())?;
		let kind = detail.formation.throw_outcropping(extent, spatial_index.noise)?;
		Some((Self::selected(extent, kind, spatial_index.noise), extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut TerrainDetailIndex,
		_lod_ref: &LodRef,
	) {
	}
}

fn region_overlaps_cell(region: Aabb3d, cell: OutcroppingExtent) -> bool {
	let bounds = cell.aabb();
	region.min.x < bounds.max.x
		&& region.max.x > bounds.min.x
		&& region.min.z < bounds.max.z
		&& region.max.z > bounds.min.z
}

/// Channel marker for terrain-detail generate / present messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct TerrainDetailLodChan;

fn outcropping_tile_index(position: Vec3) -> (i32, i32) {
	OutcroppingExtent::cell_index_containing(position)
}

/// Generate bullseye: emit a metric AABB when the driver crosses a 40 m cell.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct TerrainDetailGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for TerrainDetailGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: TERRAIN_DETAIL_GENERATE_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for TerrainDetailGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = outcropping_tile_index(lod_ref.previous_transform.translation);
		let current = outcropping_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(FormationExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

/// Present bullseye — typically closer than generate.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct TerrainDetailPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for TerrainDetailPresentBullseye {
	fn default() -> Self {
		Self { radius_m: TERRAIN_DETAIL_PRESENT_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for TerrainDetailPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = outcropping_tile_index(lod_ref.previous_transform.translation);
		let current = outcropping_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(FormationExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		FormationKind, OutcroppingKind, DEFAULT_FORMATION_EXTENT_XZ, DEFAULT_OUTCROPPING_EXTENT_XZ,
	};
	use anyhow::Result;
	use lod::gen::GeneratingSpatialIndex;

	fn test_lod_ref(bounds: Aabb3d) -> (Transform, Aabb3d) {
		(Transform::IDENTITY, bounds)
	}

	#[test]
	fn detail_original_ids_are_overlapping_formation_cells() -> Result<()> {
		let region = FormationExtent::xz_radius_aabb(Vec3::new(200.0, 0.0, 200.0), 10.0);
		let ids = TerrainDetail::original_ids_for(&mut TerrainDetailIndex::default(), region);
		assert_eq!(ids.len(), 1);
		Ok(())
	}

	#[test]
	fn detail_build_is_select_only() -> Result<()> {
		let mut index = TerrainDetailIndex::default();
		index.formation = Some(FormationKind::BoulderField);
		let extent = FormationExtent::from_cell_index(0, 0);
		let (identity, bounds) = test_lod_ref(extent.aabb());
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<TerrainDetail>::get_or_generate(
			&mut index,
			extent.id(),
			&lod_ref
		)
		.is_some());
		let detail = SpatialIndex::<TerrainDetail>::get(&index, extent.id())
			.ok_or_else(|| anyhow::anyhow!("detail"))?;
		assert_eq!(detail.formation, FormationKind::BoulderField);
		Ok(())
	}

	#[test]
	fn empty_formation_emits_no_outcropping_origins() -> Result<()> {
		let mut index = TerrainDetailIndex::default();
		index.formation = Some(FormationKind::Empty);
		let region = FormationExtent::from_cell_index(0, 0).aabb();
		let ids = TerrainOutcropping::original_ids_for(&mut index, region);
		assert!(ids.is_empty());
		assert!(SpatialIndex::<TerrainDetail>::get(
			&index,
			FormationExtent::from_cell_index(0, 0).id()
		)
		.is_some());
		Ok(())
	}

	#[test]
	fn boulder_field_origins_are_sparse_patches() -> Result<()> {
		let mut index = TerrainDetailIndex::default();
		index.formation = Some(FormationKind::BoulderField);
		let formation = FormationExtent::from_cell_index(0, 0);
		let ids = TerrainOutcropping::original_ids_for(&mut index, formation.aabb());
		assert!(!ids.is_empty());
		assert!(ids.len() < 100, "pinned BoulderField must not fill every 40 m cell");
		for OriginalId(id) in &ids {
			let extent = OutcroppingExtent::from_id(*id).ok_or_else(|| anyhow::anyhow!("id"))?;
			assert_eq!(
				FormationKind::BoulderField.throw_outcropping(extent, index.noise),
				Some(OutcroppingKind::BoulderPatch)
			);
		}
		Ok(())
	}

	#[test]
	fn crag_complex_origins_are_mostly_empty() -> Result<()> {
		let mut index = TerrainDetailIndex::default();
		index.formation = Some(FormationKind::CragComplex);
		let formation = FormationExtent::from_cell_index(0, 0);
		let ids = TerrainOutcropping::original_ids_for(&mut index, formation.aabb());
		assert!(ids.len() < 50, "crag veins should leave most 40 m cells empty");
		Ok(())
	}

	#[test]
	fn outcropping_build_depends_on_parent_and_does_not_grow() -> Result<()> {
		let mut index = TerrainDetailIndex::default();
		index.formation = Some(FormationKind::BoulderField);
		let formation = FormationExtent::from_cell_index(0, 0);
		let ids = TerrainOutcropping::original_ids_for(&mut index, formation.aabb());
		let id = ids.first().ok_or_else(|| anyhow::anyhow!("origin"))?.0;
		let (identity, bounds) = test_lod_ref(formation.aabb());
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<TerrainOutcropping>::get_or_generate(
			&mut index, id, &lod_ref
		)
		.is_some());
		let outcropping = SpatialIndex::<TerrainOutcropping>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("outcropping"))?;
		assert_eq!(outcropping.kind, OutcroppingKind::BoulderPatch);
		assert!(outcropping.grown_placements().is_none());
		assert!(SpatialIndex::<TerrainDetail>::get(&index, formation.id()).is_some());
		Ok(())
	}

	#[test]
	fn radii_keep_formation_and_outcropping_scales() -> Result<()> {
		assert!((TERRAIN_DETAIL_PRESENT_RADIUS_M - 800.0).abs() < 1e-3);
		assert!((TERRAIN_DETAIL_GENERATE_RADIUS_M - 1600.0).abs() < 1e-3);
		assert!((DEFAULT_FORMATION_EXTENT_XZ - 400.0).abs() < 1e-3);
		assert!((DEFAULT_OUTCROPPING_EXTENT_XZ - 40.0).abs() < 1e-3);
		Ok(())
	}
}
