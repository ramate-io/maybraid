//! ValleyTrain stamp construction on each guillotine leaf.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::valley_chain::config::JerseyValleyChainLayerConfig;
use crate::terrain::valley_chain::guillotine_cell::{
	original_ids_for_guillotine_leaves, JerseyValleyChainGuillotineCell,
};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::{JerseyModulation, ValleyTrain};
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

/// Stamp output for one ValleyChain guillotine leaf.
#[derive(Debug, Clone, Component)]
pub struct JerseyValleyChainStampCell {
	pub cell: Aabb3d,
	pub modulations: Vec<JerseyModulation>,
}

impl JerseyValleyChainStampCell {
	pub fn from_deps(
		cell: Aabb3d,
		base: &BaseTerrainNoise,
		config: &JerseyValleyChainLayerConfig,
		leaf_index: u32,
	) -> Self {
		let seed = base
			.seed
			.wrapping_add(cell_salt(cell))
			.wrapping_add(77)
			.wrapping_add(leaf_index);
		let height = |x: f32, z: f32| base.height_at(x, z);
		let height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height);
		let train = ValleyTrain::from_bounds(
			bounds2(cell),
			seed,
			config.valley_train,
			height_at,
		);
		Self {
			cell,
			modulations: train.stamp.modulations,
		}
	}
}

impl<S> GenerationScheme<S> for JerseyValleyChainStampCell
where
	S: GeneratingSpatialIndex<JerseyValleyChainGuillotineCell>
		+ GeneratingSpatialIndex<JerseyValleyChainLayerConfig>
		+ GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<crate::terrain::valley_chain::controller::JerseyValleyChainControllerCell>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_guillotine_leaves(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		GeneratingSpatialIndex::<JerseyValleyChainGuillotineCell>::get_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?;
		let leaf = <S as SpatialIndex<JerseyValleyChainGuillotineCell>>::get(spatial_index, id)?;
		let leaf_index = leaf.leaf_index;

		GeneratingSpatialIndex::<JerseyValleyChainLayerConfig>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let config = <S as SpatialIndex<JerseyValleyChainLayerConfig>>::get(
			spatial_index,
			Id::Universal,
		)?
		.clone();

		GeneratingSpatialIndex::<BaseTerrainNoise>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let base =
			<S as SpatialIndex<BaseTerrainNoise>>::get(spatial_index, Id::Universal)?.clone();

		Some((Self::from_deps(bounds, &base, &config, leaf_index), bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
