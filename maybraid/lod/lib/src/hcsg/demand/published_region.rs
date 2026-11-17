//! Read already-published values independently of generation demand.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::shape::Aabb3d;

use crate::gen::Id;
use crate::hcsg::demand::HcsgDemand;
use crate::hcsg::{Busy, HcsgStorage, HcsgValue, StoredEntry};

const READ_ENTRIES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedReadError {
	Busy,
	Stale,
}

impl From<Busy> for PublishedReadError {
	fn from(_: Busy) -> Self {
		Self::Busy
	}
}

/// Cache-read completeness, independent of any generation request's completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishedRead {
	pub complete: bool,
	pub epoch: u64,
}

/// A nonblocking published-region snapshot bound to one storage/demand pair.
///
/// Available entries may be consumed after every successful read, even while
/// generation or this read's pagination is incomplete. At most 32 Arcs are
/// cloned per read. Region ID discovery uses the existing spatial index, whose
/// sparse/unindexed fallback may scan the typed store; that query's CPU work is
/// not bounded by the Arc limit. Its ID list is reused through a pass and on
/// quiet complete reads. Membership changes restart only after the current pass,
/// so continuous publication cannot starve later IDs. Old entries may remain
/// visible during a pass; refreshed passes remove deleted IDs and update versions.
pub struct PublishedRegion<T> {
	storage: HcsgStorage,
	demand: HcsgDemand,
	region: Aabb3d,
	epoch: Option<u64>,
	revision: Option<u64>,
	ids: Vec<Id>,
	cursor: usize,
	entries: BTreeMap<Id, StoredEntry<Arc<T>>>,
}

impl<T: HcsgValue> PublishedRegion<T> {
	pub fn new(storage: HcsgStorage, demand: HcsgDemand, region: Aabb3d) -> Self {
		Self {
			storage,
			demand,
			region,
			epoch: None,
			revision: None,
			ids: Vec::new(),
			cursor: 0,
			entries: BTreeMap::new(),
		}
	}

	/// Consumers replacing either resource must create a fresh readout.
	pub fn is_for(&self, storage: &HcsgStorage, demand: &HcsgDemand) -> bool {
		self.storage.same_storage(storage) && Arc::ptr_eq(&self.demand.0, &demand.0)
	}

	/// Valid after a successful `try_read`; Busy/Stale never validate a snapshot.
	pub fn entries(&self) -> impl Iterator<Item = (&Id, &StoredEntry<Arc<T>>)> {
		self.entries.iter()
	}

	fn clear(&mut self) {
		self.revision = None;
		self.ids.clear();
		self.cursor = 0;
		self.entries.clear();
	}

	pub fn try_read(&mut self) -> Result<PublishedRead, PublishedReadError> {
		let epoch = self.demand.try_lock()?.epoch;
		if self.epoch != Some(epoch) {
			self.clear();
			self.epoch = Some(epoch);
		}
		let (revision, end, fresh) = if let Some(store) = self.storage.try_store::<T>()? {
			let nodes = store.try_read()?;
			let revision = nodes.membership_revision();
			if self.revision.is_none()
				|| (self.cursor >= self.ids.len() && self.revision != Some(revision))
			{
				self.ids = nodes.overlapping(self.region);
				// Universal is appended by NodeStore; normalize before pagination.
				self.ids.sort_unstable();
				self.ids.dedup();
				self.cursor = 0;
				self.revision = Some(revision);
				self.entries.retain(|id, _| self.ids.binary_search(id).is_ok());
			}
			let end = (self.cursor + READ_ENTRIES).min(self.ids.len());
			let fresh = self.ids[self.cursor..end]
				.iter()
				.filter_map(|id| nodes.entry(*id).cloned().map(|entry| (*id, entry)))
				.collect::<Vec<_>>();
			(revision, end, fresh)
		} else {
			self.clear();
			self.revision = Some(0);
			(0, 0, Vec::new())
		};
		// Epoch changes can race the unlocked storage phase. Never accept old
		// snapshots, including entries held by an earlier successful read.
		let current_epoch = self.demand.try_lock()?.epoch;
		if current_epoch != epoch {
			self.clear();
			self.epoch = None;
			return Err(PublishedReadError::Stale);
		}
		self.cursor = end;
		self.entries.extend(fresh);
		Ok(PublishedRead {
			complete: self.cursor >= self.ids.len() && self.revision == Some(revision),
			epoch,
		})
	}
}

#[cfg(test)]
mod tests;
