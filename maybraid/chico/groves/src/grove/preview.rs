//! Demo cell grids for grove render previews.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use gimme_gen::Cell;

use crate::braid_grass::BraidGrassDefinition;

/// Square grid of axis-aligned vegetation cells for playground previews.
///
/// `cell_extent` is the authored grove cell footprint (see [`BraidGrassDefinition::preview_cell_extent`]).
pub fn preview_cell_grid(cells_per_axis: u32, cell_extent: f32) -> Vec<Cell> {
	let count = cells_per_axis.max(1);
	let extent = cell_extent.max(0.1);
	let mut cells = Vec::with_capacity((count * count) as usize);
	for x in 0..count {
		for z in 0..count {
			let origin = Vec3::new(x as f32 * extent, 0.0, z as f32 * extent);
			cells.push(Cell(Aabb3d::from_min_max(origin, origin + Vec3::new(extent, 1.0, extent))));
		}
	}
	cells
}

/// Preview grid using the Braid Grass definition's authored cell footprint.
pub fn braid_grass_preview_cells(cells_per_axis: u32) -> Vec<Cell> {
	preview_cell_grid(cells_per_axis, BraidGrassDefinition::preview_cell_extent())
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn preview_cell_grid_spans_requested_count() -> Result<()> {
		let cells = preview_cell_grid(3, BraidGrassDefinition::preview_cell_extent());
		assert_eq!(cells.len(), 9);
		Ok(())
	}
}
