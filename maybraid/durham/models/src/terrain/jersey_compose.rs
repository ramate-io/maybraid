//! Aggregates independent jersey family layers into one modulation list per jersey cell.

use crate::terrain::cell::{original_ids_for_jersey_cells, JerseyStampCellLayout};
use crate::terrain::jersey_layers::{
	CanyonLayer, PlateauCapLayer, PocketWaterLayer, RollingGroundLayer, RuggedMassifLayer,
	ValleyBasinLayer,
};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::JerseyModulation;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

/// Flattened jersey height ops for one jersey stamp cell (all families coexist).
#[derive(Debug, Clone, Component)]
pub struct JerseyModulations {
	pub cell: Aabb3d,
	pub modulations: Vec<JerseyModulation>,
}

impl JerseyModulations {
	fn append_layer(dst: &mut Vec<JerseyModulation>, src: &[JerseyModulation]) {
		dst.extend(src.iter().cloned());
	}
}

impl<S> GenerationScheme<S> for JerseyModulations
where
	S: GeneratingSpatialIndex<ValleyBasinLayer>
		+ GeneratingSpatialIndex<PlateauCapLayer>
		+ GeneratingSpatialIndex<RuggedMassifLayer>
		+ GeneratingSpatialIndex<CanyonLayer>
		+ GeneratingSpatialIndex<PocketWaterLayer>
		+ GeneratingSpatialIndex<RollingGroundLayer>
		+ GeneratingSpatialIndex<JerseyStampCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_jersey_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let mut modulations = Vec::new();

		GeneratingSpatialIndex::<ValleyBasinLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<ValleyBasinLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		GeneratingSpatialIndex::<PlateauCapLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<PlateauCapLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		GeneratingSpatialIndex::<RuggedMassifLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<RuggedMassifLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		GeneratingSpatialIndex::<CanyonLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<CanyonLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		GeneratingSpatialIndex::<PocketWaterLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<PocketWaterLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		GeneratingSpatialIndex::<RollingGroundLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<RollingGroundLayer>>::get(spatial_index, id) {
			Self::append_layer(&mut modulations, &layer.modulations);
		}

		Some((Self { cell: bounds, modulations }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
