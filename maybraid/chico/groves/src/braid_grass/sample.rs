//! Deterministic sampling from authored [`BraidGrassClump`] ranges.

use std::ops::RangeInclusive;

use bevy_math::Vec3;
use chico_ball_components::tuft::BladeTuftShape;
use procedural_common::UnitRange;

use crate::braid_grass::BraidGrassClump;

/// Stable unit sample in `[0, 1)` from world position and salt.
pub fn unit_from_position(position: Vec3, salt: u32) -> f32 {
	let mixed = salt
		.wrapping_mul(0x9E37_79B9)
		.wrapping_add(position.x.to_bits())
		.wrapping_add(position.y.to_bits().rotate_left(3))
		.wrapping_add(position.z.to_bits().rotate_left(7));
	(mixed as f32) / (u32::MAX as f32)
}

fn sample_unit_range(range: UnitRange, unit: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	lo + unit.clamp(0.0, 1.0) * (hi - lo)
}

fn sample_u32_range(range: &RangeInclusive<u32>, unit: f32) -> u32 {
	let lo = *range.start();
	let hi = *range.end();
	lo + (unit.clamp(0.0, 1.0) * (hi - lo) as f32).round() as u32
}

/// Sample a [`BladeTuftShape`] from authored braid-grass ranges at `position`.
pub fn blade_tuft_shape_from(position: Vec3, grass: &BraidGrassClump, foliage_seed: i32) -> BladeTuftShape {
	let height = sample_unit_range(grass.height, unit_from_position(position, 1));
	let width = sample_unit_range(grass.width, unit_from_position(position, 2));
	let blade_count = sample_u32_range(&grass.blade_count, unit_from_position(position, 3));
	let max_tilt =
		sample_unit_range(grass.braid_twist, unit_from_position(position, 4)).max(0.01);

	BladeTuftShape {
		blade_count,
		blade_length: height.max(0.1),
		blade_width: width.max(0.005),
		max_tilt_radians: max_tilt,
		seed: foliage_seed,
		..BladeTuftShape::default()
	}
}
