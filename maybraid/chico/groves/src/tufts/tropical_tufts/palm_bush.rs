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
	use chico_sbs_geometry::sbs::palm_bush::{PalmBushCrownParams, PalmBushScale};
	use chico_sbs_geometry::PalmBushSbs;
	use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};

	use super::TropicalPalmBush;

	impl BuildWithNoise<PalmBushSbs> for TropicalPalmBush {
		fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
			/*let mut noise = noise;
			noise.frequency = 10.0;
			let height_sample = noise.build().sample_unit_3d(0.0, 0.0, 0.0);
			let height = self.height.start.min(self.height.end)
				+ height_sample * (self.height.end - self.height.start);*/
			PalmBushSbs::default().with_height(2.4).with_frond_world_scale(0.6)
		}
	}
}
