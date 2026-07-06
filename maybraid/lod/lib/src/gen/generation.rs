use crate::gen::id::Id;
use crate::gen::spatial_index::SpatialIndex;
use crate::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;

pub trait BuildWithIdLod<S>: Sized {
	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)>;

	fn descendants_with_lod(id: Id, spatial_index: &mut S, lod_ref: &LodRef);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterializeStatus {
	Existing,
	Created,
}

pub trait GeneratingSpatialIndex<T>: SpatialIndex<T>
where
	T: BuildWithIdLod<Self>,
	Self: Sized,
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
		let ids = self.deduplicated_ids_for(region);

		ids.into_iter()
			.filter_map(|id| {
				self.get_or_generate(id, lod_ref)?;
				self.get_bounds(id).map(|bounds| (id, bounds))
			})
			.collect()
	}
}

impl<T, S> GeneratingSpatialIndex<T> for S
where
	S: SpatialIndex<T>,
	T: BuildWithIdLod<S>,
{
}
