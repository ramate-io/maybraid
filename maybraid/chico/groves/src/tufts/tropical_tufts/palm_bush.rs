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
	use chico_sbs_geometry::PalmBushSbs;
	use procedural_common::{BuildWithNoise, NoiseParams};

	use super::TropicalPalmBush;

	impl BuildWithNoise<PalmBushSbs> for TropicalPalmBush {
		fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
			// TODO: sample the authored `height` / `frond_count` / `frond_length` /
			// `crown_spread` ranges from `noise` instead of fixed companion values.
			PalmBushSbs::default()
				.with_height(2.4)
				.with_frond_world_scale(0.6)
				.with_noise_params(noise)
		}
	}
}
