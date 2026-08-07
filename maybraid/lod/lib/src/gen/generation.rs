//! Pure materialization. Generation walks dependencies and descendants
//! through whatever `S` it is handed and only ever mutates the spatial index.
//! It knows nothing about scenes; presentation is a separate pass
//! (see [`crate::gen::presentation`]).

#[cfg(test)]
mod tests;

use crate::gen::id::{Id, OriginalId, StorageStatus};
use crate::gen::spatial_index::SpatialIndex;
use crate::scene::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;
use std::collections::HashSet;

/// A type's generation scheme: which ids originate in a region, how to build
/// an instance, and which descendants to materialize alongside it.
///
/// `S` is the spatial store the scheme runs against. Dependencies and
/// descendants recurse through the same `S` (typically via
/// [`GeneratingSpatialIndex`] bounds), so the whole tree materializes from a
/// single entry point.
pub trait GenerationScheme<S>: Sized {
	/// Ids that originate in the region for this type.
	///
	/// Mutable because computing origins may itself require generating and
	/// inserting dependencies.
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId>;

	/// Builds the instance, materializing any dependencies through `S`.
	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)>;

	/// Materializes descendants through `S` for the given LOD.
	///
	/// Typically, this will materialize the next descendant type, allow that to recurse.
	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterializeStatus {
	Existing,
	Created,
}

/// A [`SpatialIndex`] lifted with a [`GenerationScheme`].
///
/// This is implemented once, blanket, for every `(T, S)` pair where `T`
/// defines a scheme over `S`. There is no separate middleware path: this is
/// the only generation algorithm.
pub trait GeneratingSpatialIndex<T>: SpatialIndex<T> {
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus>;

	/// Materialize `id` if needed, then return the stored entry.
	fn get_one_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<&T> {
		self.get_or_generate(id, lod_ref)?;
		self.get(id)
	}

	/// Materializes everything originating or tracked in the region and
	/// returns the ids with their bounds.
	fn get_or_generate_region(&mut self, region: Aabb3d, lod_ref: &LodRef) -> Vec<(Id, Aabb3d)>;

	/// Like [`Self::get_or_generate_region`], but returns stored values (skips misses).
	fn get_or_generate_region_values(&mut self, region: Aabb3d, lod_ref: &LodRef) -> Vec<&T> {
		let ids: Vec<Id> = self
			.get_or_generate_region(region, lod_ref)
			.into_iter()
			.map(|(id, _)| id)
			.collect();
		ids.into_iter().filter_map(|id| self.get(id)).collect()
	}
}

impl<T, S> GeneratingSpatialIndex<T> for S
where
	S: SpatialIndex<T>,
	T: GenerationScheme<S>,
{
	fn get_or_generate(&mut self, id: Id, lod_ref: &LodRef) -> Option<MaterializeStatus> {
		if self.get(id).is_some() {
			return Some(MaterializeStatus::Existing);
		}

		let (instance, bounds) = T::build_with_id(self, id, lod_ref)?;
		self.insert(id, instance, bounds, lod_ref);
		T::descendants_with_lod(id, self, lod_ref);

		Some(MaterializeStatus::Created)
	}

	fn get_or_generate_region(&mut self, region: Aabb3d, lod_ref: &LodRef) -> Vec<(Id, Aabb3d)> {
		// Fresh origins (not tracked anywhere; ids tracked outside the region
		// are moved assets and must not be regenerated here) plus everything
		// already tracked in the region, deduplicated.
		let mut ids: HashSet<Id> = T::original_ids_for(self, region)
			.into_iter()
			.map(|OriginalId(id)| id)
			.filter(|id| self.storage_status(*id) == StorageStatus::NotTracked)
			.collect();
		ids.extend(self.tracked_ids_for(region).into_iter().map(|tracked| tracked.0));

		let mut out: Vec<(Id, Aabb3d)> = ids
			.into_iter()
			.filter_map(|id| {
				self.get_or_generate(id, lod_ref)?;
				self.get_bounds(id).map(|bounds| (id, bounds))
			})
			.collect();
		// Deterministic compose order across neighboring queries (HashSet is unordered).
		out.sort_by(|(a, _), (b, _)| a.cmp(b));
		out
	}
}
