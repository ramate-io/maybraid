//! Noise-driven segment length from a [`std::ops::Range<f32>`].

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

/// Sample a stick length for the edge to child `child_index`.
pub fn sample_f32(
	noise: &NoiseConfig,
	range: Range<f32>,
	parent: &BallStickNode,
	segment_index: usize,
	child_index: u32,
) -> f32 {
	noise.sample_range_f32_4d(
		range.start,
		range.end,
		parent.position.x + 3.0,
		parent.position.y,
		parent.position.z,
		segment_index as f32 + child_index as f32 * 0.19,
	)
}
