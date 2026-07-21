//! Lake leaf stamps under pocket guillotine leaves ([RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake)).

use crate::terrain::cell::{cell_bounds, TerrainCellLayout};
use crate::terrain::jersey::shared::{leaf_selected, occupancy_seed};
use crate::terrain::marazion::config::MarazionWatershedConfigs;
use crate::terrain::marazion::pocket::PocketCell;
use crate::terrain::PreWatershedTerrain;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::JerseyModulation;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::{Lake, WaterFill};
use procedural_common::Bounds2;

/// Leaf stamp: Marazion lake modulations + fills.
#[derive(Debug, Clone, Component)]
pub struct MarazionLakeCell {
	pub cell: Aabb3d,
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
}

pub fn original_ids_for_marazion_lake_leaves<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<PocketCell>,
{
	crate::terrain::jersey::shared::original_ids_for_leaves::<S, PocketCell>(spatial_index, region)
}

fn pre_watershed_height_at<S>(
	spatial_index: &mut S,
	x: f32,
	z: f32,
	lod_ref: &LodRef,
) -> Option<f32>
where
	S: GeneratingSpatialIndex<PreWatershedTerrain> + GeneratingSpatialIndex<TerrainCellLayout>,
{
	let layout = GeneratingSpatialIndex::<TerrainCellLayout>::get_one_or_generate(
		spatial_index,
		Id::Universal,
		lod_ref,
	)?;
	let size = layout.cell_size.max(1e-3);
	let ix = (x / size).floor() as i32;
	let iz = (z / size).floor() as i32;
	let cell = cell_bounds(ix, iz, size, layout.vertical_half_extent);
	let id = Id::from_cell(cell);
	let pre = GeneratingSpatialIndex::<PreWatershedTerrain>::get_one_or_generate(
		spatial_index,
		id,
		lod_ref,
	)?;
	Some(pre.sdf.terrain.height_at_with_all_modulations(x, z))
}

impl<S> GenerationScheme<S> for MarazionLakeCell
where
	S: GeneratingSpatialIndex<MarazionWatershedConfigs>
		+ GeneratingSpatialIndex<PocketCell>
		+ GeneratingSpatialIndex<PreWatershedTerrain>
		+ GeneratingSpatialIndex<TerrainCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_marazion_lake_leaves(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let configs = GeneratingSpatialIndex::<MarazionWatershedConfigs>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?
		.clone();

		let occ_seed = occupancy_seed(configs.seed, 0, 0x127);
		if !leaf_selected(
			cell,
			occ_seed,
			configs.leaf_likelihood,
			configs.spatial_correlation,
		) {
			return Some((
				Self {
					cell,
					modulations: Vec::new(),
					fills: Vec::new(),
				},
				cell,
			));
		}

		let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
		let seed = configs.seed.wrapping_add(
			cell.min.x.to_bits().wrapping_mul(73856093)
				^ cell.min.z.to_bits().wrapping_mul(19349663),
		);

		let lake_c = Lake::planned_center(bounds, seed, configs.lake);
		let pre_h =
			pre_watershed_height_at(spatial_index, lake_c.x, lake_c.y, lod_ref).unwrap_or(0.0);
		let height_fn = |_: f32, _: f32| pre_h;
		let height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height_fn);

		let lake = Lake::from_bounds(bounds, seed, configs.lake, height_at);
		if lake.is_empty() {
			return Some((
				Self {
					cell,
					modulations: Vec::new(),
					fills: Vec::new(),
				},
				cell,
			));
		}
		let modulations = JerseyModulation::bind_all(lake.modulations, bounds);
		Some((
			Self {
				cell,
				modulations,
				fills: lake.fills,
			},
			cell,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
