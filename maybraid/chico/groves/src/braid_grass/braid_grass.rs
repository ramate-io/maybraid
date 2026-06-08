//! Per-clump Braid Grass construction parameters ([RFC-183 §3.4.5.1] geometry fields).

use std::ops::RangeInclusive;

use procedural_common::UnitRange;

/// Authored geometry ranges for one braid-grass clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassClump {
	pub height: UnitRange,
	pub width: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub braid_twist: UnitRange,
}
