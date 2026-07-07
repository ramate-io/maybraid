//! Storage truth. A [`SpatialIndex`] tracks values, bounds, and versions by
//! [`Id`]. It never generates and never presents.

use crate::gen::id::{Id, StorageStatus, TrackedId};
use crate::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;

/// Monotonic per-index storage version.
///
/// Stamped by the index on every insert (including re-inserts that overwrite
/// an id). Presenters compare a stored version against the version they last
/// presented per id, so "genuinely new" and "changed since presented" are
/// knowable from data alone — no commit phase or transient event handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub u64);

pub trait SpatialIndex<T> {
	/// Ids whose current bounds intersect the region.
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId>;

	/// Whether the id is untracked, tracked within its origin region, or
	/// tracked but currently living elsewhere (a moved asset).
	fn storage_status(&self, id: Id) -> StorageStatus;

	fn get(&self, id: Id) -> Option<&T>;

	fn get_bounds(&self, id: Id) -> Option<Aabb3d>;

	/// The storage version stamped when the id was last inserted.
	fn version(&self, id: Id) -> Option<Version>;

	/// Inserts the value, stamping a fresh [`Version`]. Must not spawn scenes.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef);
}
