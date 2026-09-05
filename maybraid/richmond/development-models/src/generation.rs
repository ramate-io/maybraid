//! [`GenerationScheme`] for development cells, padded terrain, and building hosts.

use bevy::log::info_span;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use durham_terrain_models::origin_cell_ids_for_layout;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_buildings::Fit;
use richmond_developments::PlacedBuilding;
use richmond_urbanization::{UrbanDevelopmentKind, UrbanizationExtent};

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
		spatial_index: &mut DevelopmentIndex<'w>,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		if !spatial_index.config().use_urbanization {
			return DevelopmentExtent::original_ids_overlapping(region);
		}
		let noise = spatial_index.config().urbanization_noise();
		spatial_index.urbanization.noise = noise;
		let mut ids = Vec::new();
		for extent in UrbanizationExtent::cells_overlapping(region) {
			spatial_index.urbanization.ensure_selected(extent, noise);
			let Some(selected) = spatial_index.urbanization.get(extent.id()) else {
				continue;
			};
			for leaf in &selected.leaves {
				if leaf.kind != UrbanDevelopmentKind::Empty
					&& region.min.x < leaf.bounds.max.x
					&& region.max.x > leaf.bounds.min.x
					&& region.min.z < leaf.bounds.max.z
					&& region.max.z > leaf.bounds.min.z
				{
					ids.push(OriginalId(leaf.id()));
				}
			}
		}
		ids
	}

	fn build_with_id(
		spatial_index: &mut DevelopmentIndex<'w>,
		id: Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		if spatial_index.config().use_urbanization {
			return build_from_urbanization_leaf(spatial_index, id);
		}
		build_from_lattice_extent(spatial_index, id)
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

fn build_from_urbanization_leaf(
	spatial_index: &mut DevelopmentIndex<'_>,
	id: Id,
) -> Option<(DevelopmentCell, Aabb3d)> {
	if spatial_index.urbanization.leaf(id).is_none() {
		let bounds = id.origin_cell_bounds()?;
		let noise = spatial_index.config().urbanization_noise();
		spatial_index.urbanization.noise = noise;
		for extent in UrbanizationExtent::cells_overlapping(bounds) {
			spatial_index.urbanization.ensure_selected(extent, noise);
		}
	}
	let leaf = spatial_index.urbanization.leaf(id)?.clone();
	let kind = DevelopmentKind::from(leaf.kind);
	let cell = leaf.bounds;
	build_development_for_kind(spatial_index, cell, kind)
}

fn build_from_lattice_extent(
	spatial_index: &mut DevelopmentIndex<'_>,
	id: Id,
) -> Option<(DevelopmentCell, Aabb3d)> {
	let extent = DevelopmentExtent::from_id(id)?;
	let cell = extent.aabb();
	let config = spatial_index.config().clone();
	let kind = select_kind(cell, &config);
	build_development_for_kind(spatial_index, cell, kind)
}

fn build_development_for_kind(
	spatial_index: &mut DevelopmentIndex<'_>,
	cell: Aabb3d,
	kind: DevelopmentKind,
) -> Option<(DevelopmentCell, Aabb3d)> {
	let layout = spatial_index.layout().clone();
	let config = spatial_index.config().clone();
	let center = Vec3::new(
		(cell.min.x + cell.max.x) * 0.5,
		(cell.min.y + cell.max.y) * 0.5,
		(cell.min.z + cell.max.z) * 0.5,
	);

	let development = match kind {
		DevelopmentKind::Empty => DevelopmentCell::empty(cell),
		DevelopmentKind::LesHalles => {
			let Some(height) =
				composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
			else {
				return Some((DevelopmentCell::empty(cell), cell));
			};
			let filled = DevelopmentCell::with_les_halles(cell, height, &config);
			let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
				terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
			});
			if overlaps_hydro {
				DevelopmentCell::empty(cell)
			} else {
				filled
			}
		}
		DevelopmentKind::ShepherdsVillage => {
			match build_shepherds_village(spatial_index.terrain_store(), &layout, cell, &config) {
				Some((village, pads)) => {
					DevelopmentCell::with_shepherds_village(cell, village, pads)
				}
				None => DevelopmentCell::empty(cell),
			}
		}
		DevelopmentKind::ShepherdsCommune => {
			match build_shepherds_commune(spatial_index.terrain_store(), &layout, cell, &config) {
				Some((commune, pads)) => {
					DevelopmentCell::with_shepherds_commune(cell, commune, pads)
				}
				None => DevelopmentCell::empty(cell),
			}
		}
		DevelopmentKind::OldCityMarket => {
			match ArchetypeGenerator::build_old_city_market(
				spatial_index.terrain_store(),
				&layout,
				cell,
				&config,
			) {
				Some((market, pads)) => DevelopmentCell::with_old_city_market(cell, market, pads),
				None => DevelopmentCell::empty(cell),
			}
		}
		DevelopmentKind::RingFort => {
			let Some(height) =
				composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
			else {
				return Some((DevelopmentCell::empty(cell), cell));
			};
			let filled = DevelopmentCell::with_ring_fort(cell, height, &config);
			let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
				terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
			});
			if overlaps_hydro {
				DevelopmentCell::empty(cell)
			} else {
				filled
			}
		}
		kind @ (DevelopmentKind::TempleComplex
		| DevelopmentKind::SingleHighrise
		| DevelopmentKind::SuburbanHomes
		| DevelopmentKind::WizardsTower
		| DevelopmentKind::SkybridgeBazaar) => {
			let Some(height) =
				composed_height_at(spatial_index.terrain_store(), &layout, center.x, center.z)
			else {
				return Some((DevelopmentCell::empty(cell), cell));
			};
			let filled = DevelopmentCell::with_archetype(cell, height, kind, &config);
			let overlaps_hydro = filled.pad_complex().is_some_and(|pad| {
				terrain_hydro_overlaps(spatial_index.terrain_store(), &layout, cell, pad.bounds)
			});
			if overlaps_hydro {
				DevelopmentCell::empty(cell)
			} else {
				filled
			}
		}
	};
	Some((development, cell))
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

		ensure_development_cells_for_bounds(spatial_index, bounds, lod_ref);
		let pads = {
			let _span = info_span!("richmond_pad_merge").entered();
			spatial_index.store.merged_pad_complex(bounds)
		};
		let padded = TerrainWithPads::compose(&terrain, [&pads]);
		Some((padded, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}

fn ensure_development_cells_for_bounds(
	spatial_index: &mut DevelopmentIndex<'_>,
	bounds: Aabb3d,
	lod_ref: &LodRef,
) {
	if spatial_index.config().use_urbanization {
		let noise = spatial_index.config().urbanization_noise();
		spatial_index.urbanization.noise = noise;
		for extent in UrbanizationExtent::cells_overlapping(bounds) {
			spatial_index.urbanization.ensure_selected(extent, noise);
		}
		let leaf_ids: Vec<Id> = spatial_index
			.urbanization
			.filled_leaves_overlapping(bounds)
			.into_iter()
			.map(|leaf| leaf.id())
			.collect();
		for leaf_id in leaf_ids {
			let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
				spatial_index,
				leaf_id,
				lod_ref,
			);
		}
		return;
	}
	for extent in DevelopmentExtent::cells_overlapping(bounds) {
		let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
			spatial_index,
			extent.id(),
			lod_ref,
		);
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
				let finish = cell.archetype()?.finish.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::TempleComplex(Box::new(
					ArchetypeGenerator::build_temple_complex(cell_aabb, &confines, noise)?
						.with_finish(finish.wall, finish.roof),
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
				let finish = cell.archetype()?.finish.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				let mut development =
					ArchetypeGenerator::build_wizards_tower(cell_aabb, confines, noise)?;
				development.building.building =
					development.building.building.with_finish(finish.wall, finish.roof);
				BuiltDevelopment::WizardsTower(Box::new(development))
			}
			DevelopmentKind::SkybridgeBazaar => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let confines = cell.confines()?;
				let connector = cell.archetype()?.finish.wall.clone();
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				BuiltDevelopment::SkybridgeBazaar(Box::new(
					ArchetypeGenerator::build_skybridge_bazaar(cell_aabb, &confines, noise)?
						.with_bridge_material(connector),
				))
			}
			DevelopmentKind::OldCityMarket => {
				let cell = SpatialIndex::<DevelopmentCell>::get(spatial_index, id)?;
				let content = cell.old_city_market()?;
				BuiltDevelopment::OldCityMarket(Box::new(content.market.clone()))
			}
		};
		Some((built, cell_aabb))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut DevelopmentIndex<'w>, _lod_ref: &LodRef) {
	}
}
