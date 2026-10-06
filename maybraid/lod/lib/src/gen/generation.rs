//! Pure materialization. Generation walks dependencies and descendants
//! through whatever `S` it is handed and only ever mutates the spatial index.
//! It knows nothing about scenes; presentation is a separate pass
//! (see [`crate::gen::presentation`]).

#[cfg(test)]
mod tests;

use crate::gen::id::{Id, OriginalId, StorageStatus};
use crate::gen::spatial_index::SpatialIndex;
use bevy::math::bounding::Aabb3d;
use std::collections::HashSet;

/// A type's generation scheme: which ids originate in a region, how to build
/// an instance, and which descendants to materialize alongside it.
///
/// `S` is the spatial store the scheme runs against. Dependencies and
/// descendants recurse through the same `S`, so the whole tree materializes
/// from a single entry point.
///
/// # Capability boundary
///
/// Bound `S` only on what this scheme touches directly: one
/// `GeneratingSpatialIndex<D>` per dependency `D` it discovers or
/// materializes (or `SpatialIndex<D>` for a plain read). Discover a
/// dependency's ids with [`GeneratingSpatialIndex::original_ids_for`] rather
/// than calling `D`'s scheme or a discovery helper, so `D`'s own layouts,
/// controllers, and configs stay `D`'s bounds. They are resolved once, at the
/// concrete index, by the blanket [`GeneratingSpatialIndex`] impl.
///
/// # No driver pose
///
/// Generation answers "build this id" independently of any camera or LOD
/// driver. Presentation decides which content it needs and requests those
/// ids. If quality changes the generated value itself, make that explicit in
/// cache identity (a separate type, id, or generation parameter), so two
/// presentation paths never share an id while expecting different content.
pub trait GenerationScheme<S>: Sized {
	/// Ids that originate in the region for this type.
	///
	/// Mutable because computing origins may itself require generating and
	/// inserting dependencies.
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId>;

	/// Builds the instance, materializing any dependencies through `S`.
	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)>;

	/// Materializes descendants that always accompany this instance.
	///
	/// Only unconditional semantic descendants belong here. Expansion driven by
	/// presentation quality belongs to the requesting presentation path.
	fn descendants(_id: Id, _spatial_index: &mut S) {}
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
	/// Ids of `T` originating in `region`, per `T`'s scheme. Lazy: nothing
	/// is materialized except what `T` needs to discover its origins.
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId>;

	fn get_or_generate(&mut self, id: Id) -> Option<MaterializeStatus>;

	/// Materialize `id` if needed, then return the stored entry.
	fn get_one_or_generate(&mut self, id: Id) -> Option<&T> {
		self.get_or_generate(id)?;
		self.get(id)
	}

	/// Visits each origin of `T` in `region` in id order, materializing one at
	/// a time. Stops with `None` at the first origin that fails to build.
	fn for_each_origin(&mut self, region: Aabb3d, mut visit: impl FnMut(&T)) -> Option<()> {
		let mut ids = GeneratingSpatialIndex::<T>::original_ids_for(self, region);
		ids.sort();
		for OriginalId(id) in ids {
			visit(GeneratingSpatialIndex::<T>::get_one_or_generate(self, id)?);
		}
		Some(())
	}

	/// Materializes everything originating or tracked in the region and
	/// returns the ids with their bounds.
	fn get_or_generate_region(&mut self, region: Aabb3d) -> Vec<(Id, Aabb3d)>;

	/// Like [`Self::get_or_generate_region`], but returns stored values (skips misses).
	fn get_or_generate_region_values(&mut self, region: Aabb3d) -> Vec<&T> {
		let ids: Vec<Id> =
			self.get_or_generate_region(region).into_iter().map(|(id, _)| id).collect();
		ids.into_iter().filter_map(|id| self.get(id)).collect()
	}
}

impl<T, S> GeneratingSpatialIndex<T> for S
where
	S: SpatialIndex<T>,
	T: GenerationScheme<S>,
{
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		T::original_ids_for(self, region)
	}

	fn get_or_generate(&mut self, id: Id) -> Option<MaterializeStatus> {
		if self.get(id).is_some() {
			return Some(MaterializeStatus::Existing);
		}

		let (instance, bounds) = T::build_with_id(self, id)?;
		self.insert(id, instance, bounds);
		T::descendants(id, self);

		Some(MaterializeStatus::Created)
	}

	fn get_or_generate_region(&mut self, region: Aabb3d) -> Vec<(Id, Aabb3d)> {
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
				self.get_or_generate(id)?;
				self.get_bounds(id).map(|bounds| (id, bounds))
			})
			.collect();
		// Deterministic compose order across neighboring queries (HashSet is unordered).
		out.sort_by(|(a, _), (b, _)| a.cmp(b));
		out
	}
}
