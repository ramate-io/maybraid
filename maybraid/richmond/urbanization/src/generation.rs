//! [`GenerationScheme`] and camera bullseyes for [`SelectedUrbanization`].

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, OriginalId};
use lod::lod_ref::LodRef;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

use crate::index::UrbanizationIndex;
use crate::{SelectedUrbanization, UrbanizationExtent};

/// Urbanization selection generate ring around the camera (metres).
pub const DEVELOPMENT_GENERATE_RADIUS_M: f32 = 3000.0;

/// Urbanization present ring around the camera (metres).
pub const DEVELOPMENT_PRESENT_RADIUS_M: f32 = 1000.0;

impl GenerationScheme<UrbanizationIndex> for SelectedUrbanization {
	fn original_ids_for(_spatial_index: &mut UrbanizationIndex, region: Aabb3d) -> Vec<OriginalId> {
		UrbanizationExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut UrbanizationIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = UrbanizationExtent::from_id(id)?;
		spatial_index.ensure_selected(extent, spatial_index.noise);
		let selected = spatial_index.get(id)?.clone();
		Some((selected, extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut UrbanizationIndex,
		_lod_ref: &LodRef,
	) {
	}
}

/// Channel marker for urbanization generate / present messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct UrbanizationLodChan;

/// Generate bullseye: emit a metric AABB when the driver crosses a 1600 m cell.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct UrbanizationGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for UrbanizationGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: DEVELOPMENT_GENERATE_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for UrbanizationGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		urbanization_ring_status(self.enabled, lod_ref, self.radius_m)
	}

	fn lod_current_region(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		urbanization_current_region(self.enabled, lod_ref, self.radius_m)
	}
}

/// Present ring — typically 1 km when generate is 3 km.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct UrbanizationPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for UrbanizationPresentBullseye {
	fn default() -> Self {
		Self { radius_m: DEVELOPMENT_PRESENT_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for UrbanizationPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		urbanization_ring_status(self.enabled, lod_ref, self.radius_m)
	}

	fn lod_current_region(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		urbanization_current_region(self.enabled, lod_ref, self.radius_m)
	}
}

fn urbanization_cell_center(position: Vec3) -> Vec3 {
	let (ix, iz) = UrbanizationExtent::cell_index_containing(position);
	UrbanizationExtent::from_cell_index(ix, iz).center()
}

fn urbanization_ring_aabb(position: Vec3, radius: f32) -> Aabb3d {
	UrbanizationExtent::xz_radius_aabb(urbanization_cell_center(position), radius)
}

fn urbanization_ring_status(
	enabled: bool,
	lod_ref: &LodRef,
	radius: f32,
) -> LodRefreshRegionsStatus {
	if !enabled {
		return LodRefreshRegionsStatus::Unchanged;
	}
	let previous =
		UrbanizationExtent::cell_index_containing(lod_ref.previous_transform.translation);
	let current = UrbanizationExtent::cell_index_containing(lod_ref.current_transform.translation);
	if current == previous {
		return LodRefreshRegionsStatus::Unchanged;
	}
	LodRefreshRegionsStatus::Changed(urbanization_ring_aabb(
		lod_ref.current_transform.translation,
		radius,
	))
}

fn urbanization_current_region(enabled: bool, lod_ref: &LodRef, radius: f32) -> Option<Aabb3d> {
	enabled.then(|| urbanization_ring_aabb(lod_ref.current_transform.translation, radius))
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::GeneratingSpatialIndex;
	use procedural_common::NoiseParams;

	fn test_lod_ref(bounds: Aabb3d) -> (bevy::prelude::Transform, Aabb3d) {
		(bevy::prelude::Transform::IDENTITY, bounds)
	}

	#[test]
	fn urbanization_original_ids_are_overlapping_cells() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 1);
		let ids = SelectedUrbanization::original_ids_for(&mut UrbanizationIndex::default(), region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}

	#[test]
	fn urbanization_build_is_select_only() -> Result<()> {
		let mut index = UrbanizationIndex::default();
		index.noise = NoiseParams::from_scalar(9.0, 0.005, 1.0, 1);
		let extent = UrbanizationExtent::default_cell();
		let id = extent.id();
		let (identity, bounds) = test_lod_ref(extent.aabb());
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<SelectedUrbanization>::get_or_generate(
			&mut index, id, &lod_ref
		)
		.is_some());
		let selected = lod::gen::SpatialIndex::<SelectedUrbanization>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("urbanization"))?;
		assert_eq!(selected.extent, extent);
		Ok(())
	}
}
