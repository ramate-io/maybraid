//! Store-size and rebuild diagnostics (`MAYBRAID_HCSG_DIAG` reads these).

use std::sync::atomic::Ordering;

use super::{Busy, HcsgStorage};

impl HcsgStorage {
	/// `(type name, len)` for every store, sorted by name, without waiting on
	/// the registry or typed stores. Busy withholds the entire diagnostic sample.
	pub fn try_store_sizes(&self) -> Result<Vec<(&'static str, usize)>, Busy> {
		let stores: Vec<_> = self.0.try_read_stores()?.values().cloned().collect();
		let mut sizes = stores
			.iter()
			.map(|store| Ok((store.type_name(), store.try_len()?)))
			.collect::<Result<Vec<_>, Busy>>()?;
		sizes.sort_by_key(|(name, _)| *name);
		Ok(sizes)
	}

	/// `(type name, len)` for every store, sorted by name. Diagnostics.
	/// Blocking; frame diagnostics should use [`Self::try_store_sizes`].
	pub fn store_sizes(&self) -> Vec<(&'static str, usize)> {
		let mut sizes: Vec<_> = self
			.0
			.read_stores()
			.values()
			.map(|store| (store.type_name(), store.len()))
			.collect();
		sizes.sort_by_key(|(name, _)| *name);
		sizes
	}

	/// Top-level and nested rebuilds of values this store previously evicted.
	pub fn rebuilds_after_eviction(&self) -> (u64, u64) {
		(
			self.0.top_rebuilds.load(Ordering::Relaxed),
			self.0.nested_rebuilds.load(Ordering::Relaxed),
		)
	}
}

#[cfg(test)]
mod tests {
	use bevy::math::Vec3;
	use bevy::shape::Aabb3d;

	use crate::hcsg::{Busy, HcsgStorage};

	fn seeded_storage() -> HcsgStorage {
		let storage = HcsgStorage::default();
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		storage.seed(1_u32, bounds);
		storage.seed(true, bounds);
		storage
	}

	#[test]
	fn store_sizes_count_each_type_without_waiting() -> anyhow::Result<()> {
		let storage = seeded_storage();
		let sizes = storage.try_store_sizes().map_err(|_| anyhow::anyhow!("storage busy"))?;
		assert_eq!(
			sizes,
			vec![(std::any::type_name::<bool>(), 1), (std::any::type_name::<u32>(), 1)]
		);
		Ok(())
	}

	#[test]
	fn store_sizes_withholds_sample_when_registry_is_busy() {
		let storage = seeded_storage();
		let _writer = storage.0.write_stores();
		assert_eq!(storage.try_store_sizes(), Err(Busy));
	}

	#[test]
	fn store_sizes_withholds_whole_sample_when_one_typed_store_is_busy() -> anyhow::Result<()> {
		let storage = seeded_storage();
		let store = storage.store::<u32>().ok_or_else(|| anyhow::anyhow!("missing u32 store"))?;
		let writer = store.write();
		assert_eq!(storage.try_store_sizes(), Err(Busy));
		drop(writer);
		assert_eq!(
			storage.try_store_sizes().map_err(|_| anyhow::anyhow!("storage busy"))?.len(),
			2
		);
		Ok(())
	}
}
