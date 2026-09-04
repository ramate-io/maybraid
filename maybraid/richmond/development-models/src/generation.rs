//! [`GenerationScheme`] for development cells, padded terrain, and building hosts.

use bevy::math::bounding::Aabb3d;
use durham_terrain_models::origin_cell_ids_for_layout;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_buildings::Fit;
use richmond_developments::PlacedBuilding;

use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::cell::DevelopmentExtent;
use crate::commune::build_shepherds_commune;
use crate::development::{select_kind, DevelopmentCell, DevelopmentKind};
use crate::hydro::{composed_height_at, terrain_hydro_overlaps};
use crate::index::DevelopmentIndex;
use crate::les_halles::LesHallesDevelopment;
use crate::padded::TerrainWithPads;
use crate::ring_fort::RingFortDevelopment;
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
			DevelopmentKind::RingFort => {
				let center = extent.center();
				let Some(height) =
					composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
				else {
					return Some((Self::empty(cell), cell));
				};
				let filled = Self::with_ring_fort(cell, height, &config);
				let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
					terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
				});
				if overlaps_hydro {
					Self::empty(cell)
				} else {
					filled
				}
			}
			kind @ (DevelopmentKind::TempleComplex
			| DevelopmentKind::SingleHighrise
			| DevelopmentKind::SuburbanHomes
			| DevelopmentKind::WizardsTower
			| DevelopmentKind::SkybridgeBazaar
			| DevelopmentKind::OldCityMarket) => {
				let center = extent.center();
				let Some(height) =
					composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
				else {
					return Some((Self::empty(cell), cell));
				};
				let filled = Self::with_archetype(cell, height, kind, &config);
				let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
					terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
				});
				if overlaps_hydro {
					Self::empty(cell)
				} else {
					filled
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

impl<'w> GenerationScheme<DevelopmentIndex<'w>> for BuiltDevelopment {
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
		let kind = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?.kind();
		let cell_aabb = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?.cell;
		let built = match kind {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => {
				let (confines, finish, footprint, yaw, ground_height) = {
					let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
					let content = cell.les_halles()?;
					(
						cell.confines()?,
						content.finish.clone(),
						content.confines_extent_xz,
						content.confines_yaw,
						content.pad.height,
					)
				};
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				let (development, _) =
					richmond_developments::MixedUseLesHallesDevelopment::fit_to_confines(
						&confines, noise,
					)
					.ok()?;
				BuiltDevelopment::LesHalles(Box::new(LesHallesDevelopment {
					cell: cell_aabb,
					building: PlacedBuilding {
						center_xz: crate::pad::cell_center_xz(cell_aabb),
						yaw,
						footprint,
						ground_height,
						building: development.with_finish(finish.wall, finish.roof),
					},
				}))
			}
			DevelopmentKind::ShepherdsVillage => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let content = cell.shepherds_village()?;
				BuiltDevelopment::ShepherdsVillage(Box::new(ShepherdsVillageDevelopment {
					village: content.village.clone(),
				}))
			}
			DevelopmentKind::ShepherdsCommune => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let content = cell.shepherds_commune()?;
				BuiltDevelopment::ShepherdsCommune(Box::new(ShepherdsCommuneDevelopment {
					commune: content.commune.clone(),
				}))
			}
			DevelopmentKind::RingFort => {
				let (confines, finish, footprint, yaw, ground_height) = {
					let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
					let content = cell.ring_fort()?;
					(
						cell.confines()?,
						content.finish.clone(),
						content.confines_extent_xz,
						content.confines_yaw,
						content.pad.height,
					)
				};
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				let (development, _) =
					richmond_developments::RingFort::fit_to_confines(&confines, noise).ok()?;
				BuiltDevelopment::RingFort(Box::new(RingFortDevelopment {
					cell: cell_aabb,
					building: PlacedBuilding {
						center_xz: crate::pad::cell_center_xz(cell_aabb),
						yaw,
						footprint,
						ground_height,
						building: development.with_finish(finish.wall, finish.roof),
					},
				}))
			}
			DevelopmentKind::TempleComplex => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let wall = cell.archetype()?.finish.wall.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::TempleComplex(Box::new(
					ArchetypeGenerator::build_temple_complex(cell_aabb, &confines, noise)?
						.with_landmark_material(wall),
				))
			}
			DevelopmentKind::SingleHighrise => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let wall = cell.archetype()?.finish.wall.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				let mut development =
					ArchetypeGenerator::build_single_highrise(cell_aabb, confines, noise)?;
				development.building.building =
					development.building.building.with_wall_material(wall);
				BuiltDevelopment::SingleHighrise(Box::new(development))
			}
			DevelopmentKind::SuburbanHomes => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::SuburbanHomes(Box::new(ArchetypeGenerator::build_suburban_homes(
					cell_aabb, &confines, noise,
				)?))
			}
			DevelopmentKind::WizardsTower => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::WizardsTower(Box::new(ArchetypeGenerator::build_wizards_tower(
					cell_aabb, confines, noise,
				)?))
			}
			DevelopmentKind::SkybridgeBazaar => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let wall = cell.archetype()?.finish.wall.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::SkybridgeBazaar(Box::new(
					ArchetypeGenerator::build_skybridge_bazaar(cell_aabb, &confines, noise)?
						.with_tower_material(wall),
				))
			}
			DevelopmentKind::OldCityMarket => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::OldCityMarket(Box::new(
					ArchetypeGenerator::build_old_city_market(cell_aabb, &confines, noise)?,
				))
			}
		};
		Some((built, cell_aabb))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}
