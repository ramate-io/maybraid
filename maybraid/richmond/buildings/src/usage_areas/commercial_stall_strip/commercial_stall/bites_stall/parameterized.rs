//! Parameterized (noise-sampled) knobs + deterministic fit for [`super::BitesStall`].

use bevy_math::bounding::Aabb3d;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::LabelStyle;

use crate::fit::{Confines, FitError};

use super::super::stall_layout::{
	eligible_bites_passages, pack_bites_counters_from_choices, pack_bites_kitchen,
	sample_bites_counter_choices, BitesCounterChoice, PackedBitesCounters, BITES_REGION_MIN_PLAN,
};

/// Noise / style knobs for [`super::BitesStall`] (sampled above; fit below).
#[derive(Debug, Clone, PartialEq)]
pub struct BitesStallParameterized {
	pub style: LabelStyle,
	/// Parallel to [`eligible_bites_passages`].
	pub counters: Vec<BitesCounterChoice>,
}

impl BitesStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let eligible = eligible_bites_passages(confines);
		if eligible.is_empty() {
			return Err(FitError::TooSmall {
				reason: "bites counter passage",
			});
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let counters = sample_bites_counter_choices(&eligible, &cfg, c, 32.0);
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 33.0));
		Ok(Self { style, counters })
	}

	/// Resolve counter AABBs from these choices (shared with sit-down).
	pub fn pack_counters(&self, confines: &Confines) -> Result<PackedBitesCounters, FitError> {
		let eligible = eligible_bites_passages(confines);
		pack_bites_counters_from_choices(confines, &eligible, &self.counters)
	}
}

/// Geometry resolved from [`BitesStallParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitesStallPlan {
	pub parameterized: BitesStallParameterized,
	pub counter_aabbs: Vec<Aabb3d>,
	pub kitchen_aabb: Aabb3d,
}

impl BitesStallPlan {
	pub fn from_parameterized(
		params: BitesStallParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.pack_counters(confines)?;
		let kitchen_aabb = pack_bites_kitchen(
			&confines.bounds,
			&packed.counters,
			&[],
			BITES_REGION_MIN_PLAN,
		)
		.ok_or(FitError::TooSmall {
			reason: "bites kitchen",
		})?;
		Ok(Self {
			parameterized: params,
			counter_aabbs: packed.counters,
			kitchen_aabb,
		})
	}
}
