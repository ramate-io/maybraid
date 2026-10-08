//! [`GenerationScheme`] for [`SelectedUrbanization`] and its producer channel.

use bevy::math::bounding::Aabb3d;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};

use crate::storage::UrbanizationSelection;
use crate::{select_cell, select_cell_as, SelectedUrbanization, UrbanizationExtent};

/// Urbanization selection generate ring around the camera (metres).
pub const DEVELOPMENT_GENERATE_RADIUS_M: f32 = 3000.0;

/// Urbanization present ring around the camera (metres).
pub const DEVELOPMENT_PRESENT_RADIUS_M: f32 = 1000.0;

/// Producer channel for urbanization selection: the generate ring around the viewer.
pub struct UrbanizationWindow;

impl<S> GenerationScheme<S> for SelectedUrbanization
where
	S: GeneratingSpatialIndex<UrbanizationSelection>,
{
	fn original_ids_for(_spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		UrbanizationExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = UrbanizationExtent::from_id(id)?;
		let selection = GeneratingSpatialIndex::<UrbanizationSelection>::get_one_or_generate(
			spatial_index,
			Id::Universal,
		)?;
		let selected = match selection.kind {
			Some(kind) => select_cell_as(extent, selection.noise, kind),
			None => select_cell(extent, selection.noise),
		};
		Some((selected, extent.aabb()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::HcsgStorage;

	#[test]
	fn urbanization_original_ids_are_overlapping_cells() -> Result<()> {
		let region = UrbanizationExtent::ring_aabb((0, 0), 1);
		let ids = HcsgStorage::default().original_ids_for::<SelectedUrbanization>(region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}
}
