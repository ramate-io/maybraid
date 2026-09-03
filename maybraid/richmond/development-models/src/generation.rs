//! [`GenerationScheme`] for development cells, padded terrain, and Les Halles.

use bevy::math::bounding::Aabb3d;
use durham_terrain_models::origin_cell_ids_for_layout;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_buildings::Fit;

use crate::cell::DevelopmentExtent;
use crate::development::{should_fill, DevelopmentCell};
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::index::DevelopmentIndex;
use crate::les_halles::LesHallesDevelopment;
use crate::pad::cell_bounds2;
use crate::padded::TerrainWithPads;

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for DevelopmentCell {
	fn original_ids_for(
		_spatial_index: &mut DevelopmentIndex<'w>,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		DevelopmentExtent::original_ids_overlapping(region)
	}

	fn build_with_id(
		spatial_index: &mut DevelopmentIndex<'w>,
		id: Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = DevelopmentExtent::from_id(id)?;
		let cell = extent.aabb();
		let bounds2 = cell_bounds2(cell);
		let layout = spatial_index.layout().clone();
		let config = spatial_index.config().clone();

		if terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, bounds2) {
			return Some((Self::empty(cell), cell));
		}

		if !should_fill(cell, &config) {
			return Some((Self::empty(cell), cell));
		}

		let center = extent.center();
		let Some(height) =
			composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
		else {
			return Some((Self::empty(cell), cell));
		};

		Some((Self::filled(cell, height, &config), cell))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for TerrainWithPads {
	fn original_ids_for(
		spatial_index: &mut DevelopmentIndex<'w>,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		origin_cell_ids_for_layout(spatial_index.layout(), region)
	}

	fn build_with_id(
		spatial_index: &mut DevelopmentIndex<'w>,
		id: Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let terrain = spatial_index.terrain_store().terrain(id)?.clone();

		for extent in DevelopmentExtent::cells_overlapping(bounds) {
			let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
				spatial_index,
				extent.id(),
				lod_ref,
			);
		}

		let mut pads = Vec::new();
		for extent in DevelopmentExtent::cells_overlapping(bounds) {
			let Some(dev) = SpatialIndex::<DevelopmentCell>::get(spatial_index, extent.id()) else {
				continue;
			};
			if let Some(modulation) = dev.pad_modulation() {
				pads.push(modulation.clone());
			}
		}

		let padded = TerrainWithPads::compose(&terrain, &pads);
		Some((padded, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for LesHallesDevelopment {
	fn original_ids_for(
		spatial_index: &mut DevelopmentIndex<'w>,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		spatial_index.store.filled_original_ids(region)
	}

	fn build_with_id(
		spatial_index: &mut DevelopmentIndex<'w>,
		id: Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(spatial_index, id, lod_ref)?;
		let seed = spatial_index.config().seed as i32;
		let (confines, cell_aabb, finish, yaw) = {
			let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
			if !cell.is_filled() {
				return None;
			}
			(cell.confines()?, cell.cell, cell.finish.clone()?, cell.confines_yaw)
		};
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		let (development, _) =
			richmond_developments::MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise)
				.ok()?;
		Some((
			Self {
				cell: cell_aabb,
				confines_yaw: yaw,
				development: development.with_finish(finish.wall, finish.roof),
			},
			cell_aabb,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}
