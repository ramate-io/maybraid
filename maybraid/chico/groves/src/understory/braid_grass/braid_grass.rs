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

#[cfg(feature = "render")]
mod render {
	use std::ops::RangeInclusive;

	use chico_ball_components::tuft::BladeTuftShape;
	use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};

	use super::BraidGrassClump;

	impl BuildWithNoise<BladeTuftShape> for BraidGrassClump {
		fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
			let config = NoiseConfig::new(noise);
			let sample_f32 = |range: UnitRange, salt| {
				let lo = range.start.min(range.end);
				let hi = range.start.max(range.end);
				config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
			};
			let sample_u32 = |range: &RangeInclusive<u32>, salt| {
				let lo = *range.start() as usize;
				let hi = (*range.end() as usize).saturating_add(1);
				config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
			};

			BladeTuftShape {
				blade_count: sample_u32(&self.blade_count, 3.0),
				blade_length: sample_f32(self.height, 1.0).max(0.1),
				blade_width: sample_f32(self.width, 2.0).max(0.005),
				max_tilt_radians: sample_f32(self.braid_twist, 4.0).max(0.01),
				seed: noise.seed,
				..BladeTuftShape::default()
			}
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		#[test]
		fn build_with_noise_respects_blade_count_range() -> Result<()> {
			let grass = BraidGrassClump {
				height: UnitRange::new(1.0, 2.0),
				width: UnitRange::new(0.3, 0.8),
				blade_count: 12..=28,
				braid_twist: UnitRange::new(0.1, 0.3),
			};
			let shape = grass.build_with_noise(NoiseParams::from_scalar(42.0, 1.0, 1.0, 1));
			assert!((12..=28).contains(&shape.blade_count));
			Ok(())
		}
	}
}
