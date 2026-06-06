use bevy_math::bounding::Aabb3d;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpatialIndexError {
	#[error("Failed to read value")]
	ReadFailed,
	#[error("Failed to insert value")]
	InsertFailed,
}

/// This is just a marker now for a typed spatial index.
pub trait TypedSpatialIndex<T> {
	fn read_one(&self, region: &Aabb3d) -> Result<Option<T>, SpatialIndexError>;

	fn insert(&mut self, value: T) -> Result<(), SpatialIndexError>;
}
