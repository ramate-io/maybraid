//! [`CellGrove`] — authored grove identity ([RFC-183 3.4]).

use bevy_math::Vec2;

use super::{distribution::GroveDistribution, params::GrovePlacementRanges};

/// Authored grove identity: cell footprint, per-draw placement ranges, and variant distribution.
pub trait CellGrove {
	type Variant: Clone;

	/// Vegetation cell span in world metres on X and Z (set by whoever grids the grove).
	///
	/// A future `cell_extent_xz` range on this trait could support variable cell sizes within one
	/// grove grid by sampling the span to the next element as cells are iterated.
	fn cell_extent_xz(&self) -> Vec2;

	/// Ranges sampled independently for each cell draw (scale, offset, foliage noise).
	fn placement_ranges(&self) -> GrovePlacementRanges;

	fn distribution(&self) -> &GroveDistribution<Self::Variant>;
}
