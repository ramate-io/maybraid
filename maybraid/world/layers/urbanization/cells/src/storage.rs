//! [`HcsgStorage`] records for urbanization selection.

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::{Id, SpatialIndex};
use lod::hcsg::{shared, HcsgStorage, StoredEntry};
use procedural_common::NoiseParams;

use crate::{
	DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationKind, DEFAULT_URBANIZATION_EXTENT_XZ,
};

/// Root input every [`SelectedUrbanization`] is thrown from.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct UrbanizationSelection {
	pub noise: NoiseParams,
	/// When set, every cell uses this kind instead of Hopscotch (playground pin).
	pub kind: Option<UrbanizationKind>,
}

lod::seeded_root!(UrbanizationSelection);

/// [`HcsgStorage`] group holding every record derived from [`UrbanizationSelection`].
pub struct UrbanizationNodes;

/// Selections are bucketed one extent (1600 m) apart.
const SELECTION_SCALE: DVec3 =
	DVec3::new(DEFAULT_URBANIZATION_EXTENT_XZ as f64, 1.0, DEFAULT_URBANIZATION_EXTENT_XZ as f64);

/// Configures the selection store and joins it to [`UrbanizationNodes`].
pub fn register_urbanization_nodes(storage: &mut HcsgStorage) {
	storage
		.configure::<SelectedUrbanization>(SELECTION_SCALE)
		.add_to_group::<UrbanizationNodes, SelectedUrbanization>();
}

impl UrbanizationNodes {
	/// Configures the selection store in the shared storage.
	pub fn configure(storage: &shared::HcsgStorage) {
		storage.configure::<SelectedUrbanization>(SELECTION_SCALE);
	}

	/// Drops every selection from the shared storage.
	pub fn clear(storage: &shared::HcsgStorage) {
		storage.clear::<SelectedUrbanization>();
	}
}

/// Urbanization reads over [`HcsgStorage`]. GET only: nothing is selected here.
pub trait UrbanizationStorage {
	fn selected(&self, id: Id) -> Option<&SelectedUrbanization>;

	/// Stored selections whose extents overlap `region`.
	fn selected_overlapping(
		&self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Id, &StoredEntry<SelectedUrbanization>)> + '_;

	/// Advances whenever a selection is stored or dropped.
	fn urbanization_revision(&self) -> u64;

	/// The stored leaf whose [`DevelopmentLeaf::id`] is `id`.
	fn leaf(&self, id: Id) -> Option<&DevelopmentLeaf>;

	/// Stored non-empty leaves whose bounds intersect `region`.
	fn filled_leaves_overlapping(&self, region: Aabb3d) -> Vec<&DevelopmentLeaf>;
}

impl UrbanizationStorage for HcsgStorage {
	fn selected(&self, id: Id) -> Option<&SelectedUrbanization> {
		self.get::<SelectedUrbanization>(id)
	}

	fn selected_overlapping(
		&self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Id, &StoredEntry<SelectedUrbanization>)> + '_ {
		self.overlapping::<SelectedUrbanization>(region)
			.into_iter()
			.filter_map(|id| self.entry::<SelectedUrbanization>(id).map(|entry| (id, entry)))
	}

	fn urbanization_revision(&self) -> u64 {
		SpatialIndex::<SelectedUrbanization>::membership_revision(self)
	}

	fn leaf(&self, id: Id) -> Option<&DevelopmentLeaf> {
		let extent = UrbanizationExtent::owning_leaf(id)?;
		self.selected(extent.id())?.leaf(id)
	}

	fn filled_leaves_overlapping(&self, region: Aabb3d) -> Vec<&DevelopmentLeaf> {
		self.selected_overlapping(region)
			.flat_map(|(_, entry)| entry.value.leaves.iter())
			.filter(|leaf| {
				leaf.kind != UrbanDevelopmentKind::Empty && region.intersects(&leaf.bounds)
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::hcsg::universal_bounds;

	fn seeded(selection: UrbanizationSelection) -> HcsgStorage {
		let mut storage = HcsgStorage::default();
		register_urbanization_nodes(&mut storage);
		storage.seed(selection, universal_bounds());
		storage
	}

	#[test]
	fn selection_is_idempotent() -> Result<()> {
		let mut storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1.0, 0.01, 1.0, 1),
			kind: None,
		});
		let id = UrbanizationExtent::default_cell().id();
		storage.get_or_generate::<SelectedUrbanization>(id);
		let first = storage.entry::<SelectedUrbanization>(id).map(|entry| entry.version);
		storage.get_or_generate::<SelectedUrbanization>(id);
		anyhow::ensure!(first.is_some(), "selection is stored");
		anyhow::ensure!(
			storage.entry::<SelectedUrbanization>(id).map(|entry| entry.version) == first,
			"a second request reuses the stored selection"
		);
		Ok(())
	}

	#[test]
	fn selection_reads_the_seeded_root() -> Result<()> {
		let mut storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::default_cell();
		let selected = storage
			.get_one_or_generate::<SelectedUrbanization>(extent.id())
			.ok_or_else(|| anyhow::anyhow!("selection"))?;
		anyhow::ensure!(selected.kind == UrbanizationKind::Frontier, "the pin is honored");
		anyhow::ensure!(selected.extent == extent);

		let mut unseeded = HcsgStorage::default();
		anyhow::ensure!(
			unseeded.get_or_generate::<SelectedUrbanization>(extent.id()).is_none(),
			"nothing is selected before the root is seeded"
		);
		Ok(())
	}

	#[test]
	fn leaf_lookup_finds_a_stored_leaf() -> Result<()> {
		let mut storage = seeded(UrbanizationSelection {
			noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
			kind: Some(UrbanizationKind::Frontier),
		});
		let extent = UrbanizationExtent::from_cell_index(1, -1);
		let leaf = storage
			.get_one_or_generate::<SelectedUrbanization>(extent.id())
			.and_then(|selected| selected.leaves.first().cloned())
			.ok_or_else(|| anyhow::anyhow!("expected leaves"))?;
		anyhow::ensure!(storage.leaf(leaf.id()) == Some(&leaf));
		Ok(())
	}
}
