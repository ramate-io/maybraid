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

/// Per-family modulation counts for one jersey stamp cell (debug / inspection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JerseyFamilySummary {
	pub name: &'static str,
	pub modulation_count: usize,
}

/// Flattened jersey height ops for one jersey stamp cell (all families coexist).
#[derive(Debug, Clone, Component)]
pub struct JerseyModulations {
	pub cell: Aabb3d,
	pub modulations: Vec<JerseyModulation>,
	/// Which stamp families contributed, and how many ops each emitted.
	pub families: Vec<JerseyFamilySummary>,
}

impl JerseyModulations {
	fn append_layer(
		dst: &mut Vec<JerseyModulation>,
		families: &mut Vec<JerseyFamilySummary>,
		name: &'static str,
		src: &[JerseyModulation],
	) {
		families.push(JerseyFamilySummary {
			name,
			modulation_count: src.len(),
		});
		dst.extend(src.iter().cloned());
	}

	/// Compact label like `V3 P2 M1 C4 W2 R3` (family initial + op count).
	pub fn family_label(&self) -> String {
		self.families
			.iter()
			.map(|f| {
				let initial = f.name.chars().next().unwrap_or('?');
				format!("{initial}{}", f.modulation_count)
			})
			.collect::<Vec<_>>()
			.join(" ")
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
		let mut families = Vec::new();

		GeneratingSpatialIndex::<ValleyBasinLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<ValleyBasinLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"valley",
				&layer.modulations,
			);
		}

		GeneratingSpatialIndex::<PlateauCapLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<PlateauCapLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"plateau",
				&layer.modulations,
			);
		}

		GeneratingSpatialIndex::<RuggedMassifLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<RuggedMassifLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"massif",
				&layer.modulations,
			);
		}

		GeneratingSpatialIndex::<CanyonLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<CanyonLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"canyon",
				&layer.modulations,
			);
		}

		GeneratingSpatialIndex::<PocketWaterLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<PocketWaterLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"water",
				&layer.modulations,
			);
		}

		GeneratingSpatialIndex::<RollingGroundLayer>::get_or_generate(spatial_index, id, lod_ref)?;
		if let Some(layer) = <S as SpatialIndex<RollingGroundLayer>>::get(spatial_index, id) {
			Self::append_layer(
				&mut modulations,
				&mut families,
				"rolling",
				&layer.modulations,
			);
		}

		Some((Self { cell: bounds, modulations, families }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
