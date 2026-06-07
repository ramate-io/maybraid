use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SpatialIndexError {
	#[error("failed to read value")]
	ReadFailed,
	#[error("failed to insert value")]
	InsertFailed,
	#[error("invalid base scale: all components must be finite and positive")]
	InvalidBaseScale,
}
