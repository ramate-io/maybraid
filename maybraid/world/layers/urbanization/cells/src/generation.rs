//! [`GenerationScheme`] for [`SelectedUrbanization`] and its producer channel.

use bevy::math::bounding::Aabb3d;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::hcsg::shared::{self, GenerationContext};

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
		Self::ids_in(region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = UrbanizationExtent::from_id(id)?;
		let selection = GeneratingSpatialIndex::<UrbanizationSelection>::get_one_or_generate(
			spatial_index,
			Id::Universal,
		)?;
		Some((Self::on(extent, selection), extent.aabb()))
	}
}

impl shared::GenerationScheme for SelectedUrbanization {
	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		Self::ids_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = UrbanizationExtent::from_id(id)?;
		let selection = cx.get_or_generate::<UrbanizationSelection>(Id::Universal)?;
		Some((Self::on(extent, &selection), extent.aabb()))
	}
}

impl SelectedUrbanization {
	fn ids_in(region: Aabb3d) -> Vec<OriginalId> {
		UrbanizationExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	/// `extent`'s cell as the session's `selection` picks it.
	fn on(extent: UrbanizationExtent, selection: &UrbanizationSelection) -> Self {
		match selection.kind {
			Some(kind) => select_cell_as(extent, selection.noise, kind),
			None => select_cell(extent, selection.noise),
		}
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

	#[test]
	fn native_selection_matches_the_legacy_one() -> Result<()> {
		use lod::hcsg::universal_bounds;
		use procedural_common::NoiseParams;

		let region = UrbanizationExtent::ring_aabb((0, 0), 2);
		for kind in [None, Some(crate::UrbanizationKind::Frontier)] {
			let selection = UrbanizationSelection {
				noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
				kind,
			};
			let mut legacy = HcsgStorage::default();
			legacy.seed(selection.clone(), universal_bounds());
			let storage = shared::HcsgStorage::default();
			storage.seed(selection, universal_bounds());
			let mut cx = GenerationContext::new(&storage);

			let ids = legacy.original_ids_for::<SelectedUrbanization>(region);
			anyhow::ensure!(ids == cx.original_ids_for::<SelectedUrbanization>(region));
			for OriginalId(id) in ids {
				let native = cx.get_or_generate::<SelectedUrbanization>(id);
				let old = legacy.get_one_or_generate::<SelectedUrbanization>(id);
				anyhow::ensure!(old == native.as_deref(), "{id:?} under {kind:?}");
			}
		}
		Ok(())
	}
}
