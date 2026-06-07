pub use gimme_core::Cell;

use bevy_math::bounding::Aabb3d;
use gimme_core::{SpatialIndexError, TypedSpatialIndex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GenerationError {
	#[error("Failed to generate value")]
	GenerationFailed(String),
	#[error("Failed on core spatial index operation")]
	SpatialIndexError(#[from] SpatialIndexError),
}

/// The base generator trait simply asks for a the implementer to provide a method for get or generating types within a requested region.
pub trait Generator<T>: TypedSpatialIndex<T> {
	/// Generates and inserts type instances intersecting with the region.  
	fn get_or_generate(
		&mut self,
		requested_region: Aabb3d,
	) -> Result<impl Iterator<Item = T>, GenerationError>;
}

/// A cellular generator narrows this by enforcing that each cell has only one value for the type.
pub trait CellGenerator<T>: TypedSpatialIndex<T>
where
	T: Clone,
{
	/// Gets all of the cells that would intersecting with the region for the type.
	fn intersecting_cells(&self, region: &Aabb3d) -> impl Iterator<Item = Cell>;

	/// Generates one instance on a cell.
	fn generate_cell(&mut self, cell: &Cell) -> Result<T, GenerationError>;

	fn get_or_generate_cell(&mut self, cell: &Cell) -> Result<&T, GenerationError> {
		if self.read_one(cell)?.is_none() {
			let value = self.generate_cell(cell)?;
			self.insert(value, cell.into_region())?;
		}

		self.read_one(cell)?.ok_or_else(|| {
			GenerationError::GenerationFailed("inserted value could not be read back".into())
		})
	}
}
