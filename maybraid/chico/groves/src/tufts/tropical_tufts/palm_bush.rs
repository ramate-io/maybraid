//! Mini palm bush companion geometry ([RFC-183 §3.4.4.5]).

use std::ops::RangeInclusive;

use procedural_common::UnitRange;

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalPalmBush {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

#[cfg(feature = "render")]
mod render {
	use std::ops::RangeInclusive;

	use bevy_math::Vec3;
	use chico_sbs_geometry::anchors::palm_bush::DEFAULT_CROWN_TUFT_SCALE_FRACTION;
	use chico_sbs_geometry::PalmBushSbs;
	use chico_sbs_geometry::sbs::palm_bush::{PalmBushCrownParams, PalmBushScale};
	use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};

	use super::TropicalPalmBush;

	impl BuildWithNoise<PalmBushSbs> for TropicalPalmBush {
		fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
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

			let height = sample_f32(self.height, 5.0).max(0.05);
			let spread = sample_f32(self.crown_spread, 3.0).max(0.1);
			let length = sample_f32(self.frond_length, 1.0).max(0.05);
			let fronds_per_ring = sample_u32(&self.frond_count, 4.0).max(4);

			PalmBushSbs {
				scale: PalmBushScale { height, base_anchor: Vec3::ZERO },
				crown: PalmBushCrownParams { ring_count: 1, fronds_per_ring },
				frond_world_scale: height.max((spread * length).max(0.15)),
				crown_tuft_scale_factor: DEFAULT_CROWN_TUFT_SCALE_FRACTION,
				foliage_noise: noise,
			}
		}
	}
}
