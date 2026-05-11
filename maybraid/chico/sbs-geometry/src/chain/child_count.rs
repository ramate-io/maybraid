//! Noise-driven child count from a half-open integer [`std::ops::Range`].

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

/// Sample how many children to spawn at a node (half-open `range`).
pub fn sample_usize(
	noise: &NoiseConfig,
	range: Range<usize>,
	parent: &BallStickNode,
	segment_index: usize,
) -> usize {
	noise.sample_range_usize_4d(
		range.start,
		range.end,
		parent.position.x,
		parent.position.y,
		parent.position.z,
		segment_index as f32,
	)
}
