//! [`HcsgStorage`] records for urbanization selection.

use bevy::math::DVec3;
use bevy::prelude::*;
use procedural_common::NoiseParams;

#[cfg(test)]
use lod::gen::Id;
#[cfg(test)]
use lod::hcsg::{Busy, HcsgStorage};

use crate::{UrbanizationKind, DEFAULT_URBANIZATION_EXTENT_XZ};

#[cfg(test)]
use crate::{DevelopmentLeaf, SelectedUrbanization, UrbanizationExtent};

/// Root input every [`SelectedUrbanization`] is thrown from.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct UrbanizationSelection {
	pub noise: NoiseParams,
	/// When set, every cell uses this kind instead of Hopscotch (playground pin).
	pub kind: Option<UrbanizationKind>,
}

lod::seeded_root!(UrbanizationSelection);

/// Selections are bucketed one extent (1600 m) apart.
pub(crate) const SELECTION_INDEX_SCALE: DVec3 =
	DVec3::new(DEFAULT_URBANIZATION_EXTENT_XZ as f64, 1.0, DEFAULT_URBANIZATION_EXTENT_XZ as f64);

/// The stored leaf whose [`DevelopmentLeaf::id`] is `id`, if its selection is
/// already published.
#[cfg(test)]
pub(crate) fn try_urbanization_leaf(
	storage: &HcsgStorage,
	id: Id,
) -> Result<Option<DevelopmentLeaf>, Busy> {
	let Some(extent) = UrbanizationExtent::owning_leaf(id) else {
		return Ok(None);
	};
	Ok(storage
		.try_entry::<SelectedUrbanization>(extent.id())?
		.map(|entry| entry.value)
		.and_then(|selected| selected.leaf(id).cloned()))
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::universal_bounds;
	use lod::hcsg::GenerationContext;

	fn seeded(selection: UrbanizationSelection) -> HcsgStorage {
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		storage
	}

	fn storage_busy(busy: Busy) -> anyhow::Error {
		anyhow::anyhow!("HcsgStorage busy: {busy:?}")
	}

	#[test]
	fn selection_is_idempotent() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1.0, 0.01, 1.0, 1),
			kind: None,
		});
		let id = UrbanizationExtent::default_cell().id();
		let mut cx = GenerationContext::new(&storage);
		cx.get_or_generate::<SelectedUrbanization>(id);
		let first = storage
			.try_entry::<SelectedUrbanization>(id)
			.map_err(storage_busy)?
			.map(|entry| entry.version);
		cx.get_or_generate::<SelectedUrbanization>(id);
		anyhow::ensure!(first.is_some(), "selection is stored");
		anyhow::ensure!(
			storage
				.try_entry::<SelectedUrbanization>(id)
				.map_err(storage_busy)?
				.map(|entry| entry.version)
				== first,
			"a second request reuses the stored selection"
		);
		Ok(())
	}

	#[test]
	fn selection_reads_the_seeded_root() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::default_cell();
		let selected = GenerationContext::new(&storage)
			.get_or_generate::<SelectedUrbanization>(extent.id())
			.ok_or_else(|| anyhow::anyhow!("selection"))?;
		anyhow::ensure!(selected.kind == UrbanizationKind::Frontier, "the pin is honored");
		anyhow::ensure!(selected.extent == extent);

		let unseeded = HcsgStorage::default();
		anyhow::ensure!(
			GenerationContext::new(&unseeded)
				.get_or_generate::<SelectedUrbanization>(extent.id())
				.is_none(),
			"nothing is selected before the root is seeded"
		);
		Ok(())
	}

	#[test]
	fn leaf_lookup_finds_a_stored_leaf() -> Result<()> {
		let storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::from_cell_index(1, -1);
		let leaf = GenerationContext::new(&storage)
			.get_or_generate::<SelectedUrbanization>(extent.id())
			.and_then(|selected| selected.leaves.first().cloned())
			.ok_or_else(|| anyhow::anyhow!("expected leaves"))?;
		anyhow::ensure!(
			try_urbanization_leaf(&storage, leaf.id()).map_err(storage_busy)? == Some(leaf)
		);
		Ok(())
	}
}
