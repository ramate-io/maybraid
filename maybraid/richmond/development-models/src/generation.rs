//! [`GenerationScheme`] for development cells, padded terrain, and building hosts.

use bevy::math::bounding::Aabb3d;
use durham_terrain_models::origin_cell_ids_for_layout;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_buildings::Fit;
use richmond_developments::PlacedBuilding;

use crate::cell::DevelopmentExtent;
use crate::commune::build_shepherds_commune;
use crate::development::{select_kind, DevelopmentCell, DevelopmentKind};
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::index::DevelopmentIndex;
use crate::les_halles::LesHallesDevelopment;
use crate::padded::TerrainWithPads;
use crate::shepherds::{ShepherdsCommuneDevelopment, ShepherdsVillageDevelopment};
use crate::village::build_shepherds_village;

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
		let layout = spatial_index.layout().clone();
		let config = spatial_index.config().clone();

		let development = match select_kind(cell, &config) {
			DevelopmentKind::Empty => Self::empty(cell),
			DevelopmentKind::LesHalles => {
				let center = extent.center();
				let Some(height) =
					composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
				else {
					return Some((Self::empty(cell), cell));
				};
				let filled = Self::with_les_halles(cell, height, &config);
				let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
					terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
				});
				if overlaps_hydro {
					Self::empty(cell)
				} else {
					filled
				}
			}
			DevelopmentKind::ShepherdsVillage => {
				match build_shepherds_village(spatial_index.terrain_store(), &layout, cell, &config)
				{
					Some((village, pads)) => Self::with_shepherds_village(cell, village, pads),
					None => Self::empty(cell),
				}
			}
			DevelopmentKind::ShepherdsCommune => {
				match build_shepherds_commune(spatial_index.terrain_store(), &layout, cell, &config)
				{
					Some((commune, pads)) => Self::with_shepherds_commune(cell, commune, pads),
					None => Self::empty(cell),
				}
			}
		};
		Some((development, cell))
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
			for complex in dev.pad_complexes() {
				pads.push(complex);
			}
		}

		let padded = TerrainWithPads::compose(&terrain, pads);
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
		let (confines, cell_aabb, finish, footprint, yaw, ground_height) = {
			let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
			let content = cell.les_halles()?;
			(
				cell.confines()?,
				cell.cell,
				content.finish.clone(),
				content.confines_extent_xz,
				content.confines_yaw,
				content.pad.height,
			)
		};
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		let (development, _) =
			richmond_developments::MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise)
				.ok()?;
		Some((
			Self {
				cell: cell_aabb,
				building: PlacedBuilding {
					center_xz: crate::pad::cell_center_xz(cell_aabb),
					yaw,
					footprint,
					ground_height,
					building: development.with_finish(finish.wall, finish.roof),
				},
			},
			cell_aabb,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for ShepherdsVillageDevelopment {
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
		let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
		let content = cell.shepherds_village()?;
		Some((Self { village: content.village.clone() }, cell.cell))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for ShepherdsCommuneDevelopment {
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
		let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
		let content = cell.shepherds_commune()?;
		Some((Self { commune: content.commune.clone() }, cell.cell))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}
