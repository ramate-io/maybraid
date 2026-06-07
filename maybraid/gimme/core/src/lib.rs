//! Gimme core: spatial index and typed storage ([RFC-142 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-142-gimme#31-spatial-index)).

mod cell;
mod error;
mod spatial_index;

pub use cell::Cell;
pub use error::SpatialIndexError;
pub use spatial_index::{
	BaseScale, HashMapStore, Level, SpatialId, SpatialIndex, SpatialStore, TypedIndex,
};

use bevy_math::bounding::Aabb3d;

/// Typed spatial query and insert over a backing store.
pub trait TypedSpatialIndex<T> {
	fn read_one(&self, region: &Aabb3d) -> Result<Option<&T>, SpatialIndexError>;

	fn insert(&mut self, value: T, bounds: Aabb3d) -> Result<&T, SpatialIndexError>;
}
