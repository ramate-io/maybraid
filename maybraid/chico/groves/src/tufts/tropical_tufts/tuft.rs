//! Tropical tuft clump geometry ([RFC-183 §3.4.4.5]).

use procedural_common::UnitRange;

/// Authored geometry ranges for one tropical tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalTuftClump {
	pub height: UnitRange,
	pub width: UnitRange,
}

#[cfg(feature = "render")]
mod render {
	use chico_ball_components::tuft::BladeTuftShape;
	use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};

	use super::TropicalTuftClump;

	impl BuildWithNoise<BladeTuftShape> for TropicalTuftClump {
		fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
			let config = NoiseConfig::new(noise);
			let sample_f32 = |range: UnitRange, salt| {
				let lo = range.start.min(range.end);
				let hi = range.start.max(range.end);
				config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
			};

			BladeTuftShape {
				blade_count: 8,
				blade_length: sample_f32(self.height, 1.0).max(0.05),
				blade_width: sample_f32(self.width, 2.0).max(0.005),
				seed: noise.seed,
				..BladeTuftShape::default()
			}
		}
	}
}
