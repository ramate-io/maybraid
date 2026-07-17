//! Independent jersey stamp family layers (coexist on each jersey cell).

use crate::terrain::cell::{original_ids_for_jersey_cells, JerseyStampCellLayout};
use crate::terrain::cell_noise::CellTerrainNoise;
use crate::terrain::jersey_configs::JerseyLayerConfigs;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::{
	Canyon, JerseyModulation, PlateauCap, PocketWater, RollingGround, RuggedMassif,
};
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::Bounds2;

fn cell_salt(cell: Aabb3d) -> u32 {
	let ix = cell.min.x.to_bits();
	let iz = cell.min.z.to_bits();
	ix.wrapping_mul(73856093) ^ iz.wrapping_mul(19349663)
}

fn bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn layer_seed(noise: &CellTerrainNoise, cell: Aabb3d, family_salt: u32) -> u32 {
	noise.seed.wrapping_add(cell_salt(cell)).wrapping_add(family_salt)
}

macro_rules! jersey_family_layer {
	(
		$layer:ident,
		$family_salt:expr,
		|$bounds:ident, $seed:ident, $height_at:ident, $configs:ident| $build:expr
	) => {
		#[derive(Debug, Clone, Component)]
		pub struct $layer {
			pub cell: Aabb3d,
			pub modulations: Vec<JerseyModulation>,
		}

		impl $layer {
			pub fn from_noise(
				cell: Aabb3d,
				noise: &CellTerrainNoise,
				configs: &JerseyLayerConfigs,
			) -> Self {
				let $bounds = bounds2(cell);
				let $seed = layer_seed(noise, cell, $family_salt);
				let $configs = configs;
				let height = |x: f32, z: f32| noise.height_at(x, z);
				let $height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height);
				let modulations = $build;
				Self { cell, modulations }
			}
		}

		impl<S> GenerationScheme<S> for $layer
		where
			S: GeneratingSpatialIndex<CellTerrainNoise>
				+ GeneratingSpatialIndex<JerseyLayerConfigs>
				+ GeneratingSpatialIndex<JerseyStampCellLayout>,
		{
			fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
				original_ids_for_jersey_cells(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: Id,
				lod_ref: &LodRef,
			) -> Option<(Self, Aabb3d)> {
				let bounds = id.origin_cell_bounds()?;
				GeneratingSpatialIndex::<JerseyLayerConfigs>::get_or_generate(
					spatial_index,
					Id::Universal,
					lod_ref,
				)?;
				let configs = <S as SpatialIndex<JerseyLayerConfigs>>::get(
					spatial_index,
					Id::Universal,
				)?
				.clone();
				GeneratingSpatialIndex::<CellTerrainNoise>::get_or_generate(
					spatial_index,
					id,
					lod_ref,
				)?;
				let noise = <S as SpatialIndex<CellTerrainNoise>>::get(spatial_index, id)?;
				Some((Self::from_noise(bounds, noise, &configs), bounds))
			}

			fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
		}
	};
}

jersey_family_layer!(PlateauCapLayer, 22, |bounds, seed, height_at, configs| {
	PlateauCap::from_bounds(bounds, seed, configs.plateau, height_at)
		.stamp
		.modulations
});

jersey_family_layer!(RuggedMassifLayer, 33, |bounds, seed, height_at, configs| {
	let _ = height_at;
	RuggedMassif::from_bounds(bounds, seed, configs.massif)
		.stamp
		.modulations
});

jersey_family_layer!(CanyonLayer, 44, |bounds, seed, height_at, configs| {
	Canyon::from_bounds(bounds, seed, configs.canyon, height_at)
		.stamp
		.modulations
});

jersey_family_layer!(PocketWaterLayer, 55, |bounds, seed, height_at, configs| {
	PocketWater::from_bounds(bounds, seed, configs.pocket_water, height_at)
		.stamp
		.modulations
});

jersey_family_layer!(RollingGroundLayer, 66, |bounds, seed, height_at, configs| {
	let _ = height_at;
	RollingGround::from_bounds(bounds, seed, configs.rolling)
		.stamp
		.modulations
});
