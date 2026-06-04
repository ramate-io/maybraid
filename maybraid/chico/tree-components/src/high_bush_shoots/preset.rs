//! Common High Bush preset constants ([#233](https://github.com/ramate-io/maybraid/issues/233), RFC §3.1.7.12).

use std::ops::RangeInclusive;

use bevy::prelude::Vec3;
use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_HEIGHT, DEFAULT_RADIAL_STRENGTH,
	DEFAULT_SEGMENT_LENGTH_FRACTION_HI, DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
	DEFAULT_SEGMENT_RADIUS_FRACTION_HI, DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
	DEFAULT_SHOOT_COUNT, DEFAULT_VERTICAL_BIAS,
};
use procedural_common::NoiseParams;

use super::config::{HighBushFoliageStyle, HighBushShootsShape};

/// RFC shoot count band for Common High Bush.
pub const COMMON_HIGH_BUSH_SHOOT_COUNT: RangeInclusive<u32> = 7..=10;

pub const COMMON_HIGH_BUSH_RADIAL_STRENGTH: f32 = DEFAULT_RADIAL_STRENGTH;
pub const COMMON_HIGH_BUSH_VERTICAL_BIAS: f32 = DEFAULT_VERTICAL_BIAS;

/// Leafy bush world radius as a fraction of `H`.
pub const COMMON_HIGH_BUSH_LEAF_RADIUS_FRACTION: f32 = 0.05;

/// Generic construction defaults before the Common High Bush preset is applied.
fn generic_high_bush_shape() -> HighBushShootsShape {
	HighBushShootsShape {
		height: DEFAULT_HEIGHT,
		base_anchor: Vec3::ZERO,
		anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
		shoot_count: 6,
		radial_strength: 0.35,
		vertical_bias: 0.55,
		branch_depth: 4,
		segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
		segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
		segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
		segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
		leaf_radius_fraction: 0.04,
		foliage_style: super::config::HighBushFoliageStyle::PlaneSplay,
		chain_noise: NoiseParams::default(),
	}
}

/// Apply RFC §3.1.7.12 defaults when fields still match the generic construction baseline.
pub fn apply_common_high_bush_preset(shape: &mut HighBushShootsShape) {
	let generic = generic_high_bush_shape();
	let preset = common_high_bush_shape();

	if shape.height == generic.height {
		shape.height = preset.height;
	}
	if shape.shoot_count == generic.shoot_count {
		shape.shoot_count = preset.shoot_count;
	}
	if (shape.radial_strength - generic.radial_strength).abs() < 1e-5 {
		shape.radial_strength = preset.radial_strength;
	}
	if (shape.vertical_bias - generic.vertical_bias).abs() < 1e-5 {
		shape.vertical_bias = preset.vertical_bias;
	}
	if (shape.anchor_lift_fraction - generic.anchor_lift_fraction).abs() < 1e-5 {
		shape.anchor_lift_fraction = preset.anchor_lift_fraction;
	}
	if shape.branch_depth == generic.branch_depth {
		shape.branch_depth = preset.branch_depth;
	}
	if (shape.segment_length_fraction_lo - generic.segment_length_fraction_lo).abs() < 1e-5 {
		shape.segment_length_fraction_lo = preset.segment_length_fraction_lo;
	}
	if (shape.segment_length_fraction_hi - generic.segment_length_fraction_hi).abs() < 1e-5 {
		shape.segment_length_fraction_hi = preset.segment_length_fraction_hi;
	}
	if (shape.leaf_radius_fraction - generic.leaf_radius_fraction).abs() < 1e-5 {
		shape.leaf_radius_fraction = preset.leaf_radius_fraction;
	}
}

pub fn common_high_bush_shape() -> HighBushShootsShape {
	HighBushShootsShape {
		height: DEFAULT_HEIGHT,
		base_anchor: Vec3::ZERO,
		shoot_count: DEFAULT_SHOOT_COUNT,
		radial_strength: COMMON_HIGH_BUSH_RADIAL_STRENGTH,
		vertical_bias: COMMON_HIGH_BUSH_VERTICAL_BIAS,
		anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
		branch_depth: 4,
		segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
		segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
		segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
		segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
		leaf_radius_fraction: COMMON_HIGH_BUSH_LEAF_RADIUS_FRACTION,
		foliage_style: HighBushFoliageStyle::PlaneSplay,
		chain_noise: NoiseParams::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn preset_shoot_count_in_rfc_band() {
		let shape = common_high_bush_shape();
		assert!(COMMON_HIGH_BUSH_SHOOT_COUNT.contains(&shape.shoot_count));
	}

	#[test]
	fn apply_preset_after_generic_cli_defaults() -> anyhow::Result<()> {
		let mut shape = generic_high_bush_shape();
		apply_common_high_bush_preset(&mut shape);
		assert_eq!(shape.shoot_count, DEFAULT_SHOOT_COUNT);
		assert!(
			(shape.radial_strength - COMMON_HIGH_BUSH_RADIAL_STRENGTH).abs() < 1e-5
		);
		Ok(())
	}

	#[test]
	fn apply_preset_preserves_explicit_overrides() -> anyhow::Result<()> {
		let mut shape = generic_high_bush_shape();
		shape.shoot_count = 12;
		apply_common_high_bush_preset(&mut shape);
		assert_eq!(shape.shoot_count, 12);
		Ok(())
	}
}
