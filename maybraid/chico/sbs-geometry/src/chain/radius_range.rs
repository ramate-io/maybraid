//! Noise-driven node radius from a [`std::ops::Range<f32>`].

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

/// Sample an end-node radius for child `child_index`.
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
		parent.position.x,
		parent.position.y + 5.0,
		parent.position.z,
		segment_index as f32 + child_index as f32 * 0.23,
	)
}
