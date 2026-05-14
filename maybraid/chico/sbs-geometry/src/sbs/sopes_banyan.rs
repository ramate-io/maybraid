//! Restricted **Sope's Banyan** geometry for CLI and playgrounds: one **`NoiseParams`** flatten, stalk, rings, and chain tuning without overlapping clap fields.

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::anchors::sopes_banyan::SopesBanyanAnchors;
use crate::anchors::strict_stalk::StrictStalk;
use crate::SopesBanyanHysteresis;

/// Streamlined config: stalk + rings + **single** canopy [`NoiseParams`] + chain height / descender threshold.
///
/// Use [`Self::to_anchors`] for the full [`SopesBanyanAnchors`] recipe and [`Self::hysteresis_seeds`] to emit seeds (wraps [`Self::canopy_noise`] in a [`NoiseConfig`]).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub stalk: StrictStalk,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub canopy_noise: NoiseParams,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 40.0))]
	pub banyan_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.01))]
	pub descender_threshold: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.40))]
	pub first_ring_unit_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.95))]
	pub last_ring_unit_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub ring_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 7))]
	pub anchors_per_ring: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.1))]
	pub projection_min_fraction_of_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.2))]
	pub projection_max_fraction_of_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.4))]
	pub vase_profile_epsilon: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.5))]
	pub projection_center_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub max_depth_first_ring: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub max_depth_last_ring: usize,
}

impl Default for SopesBanyanSbs {
	fn default() -> Self {
		Self {
			stalk: StrictStalk {
				stalk_height: 20.0,
				stalk_base_anchor: Vec3::ZERO,
				stalk_base_radius: 0.75,
			},
			canopy_noise: NoiseParams::default(),
			banyan_height: 40.0,
			descender_threshold: 0.01,
			first_ring_unit_height: 0.40,
			last_ring_unit_height: 0.95,
			ring_count: 8,
			anchors_per_ring: 7,
			projection_min_fraction_of_height: 0.1,
			projection_max_fraction_of_height: 0.2,
			vase_profile_epsilon: 0.4,
			projection_center_fraction: 0.5,
			max_depth_first_ring: 4,
			max_depth_last_ring: 8,
		}
	}
}

impl SopesBanyanSbs {
	/// Full anchor recipe (stalk + rings); chain noise is **not** duplicated here—use [`Self::hysteresis_seeds`].
	pub fn to_anchors(&self) -> SopesBanyanAnchors {
		SopesBanyanAnchors {
			stalk: self.stalk.clone(),
			descender_threshold: self.descender_threshold,
			first_ring_unit_height: self.first_ring_unit_height,
			last_ring_unit_height: self.last_ring_unit_height,
			ring_count: self.ring_count,
			anchors_per_ring: self.anchors_per_ring,
			projection_min_fraction_of_height: self.projection_min_fraction_of_height,
			projection_max_fraction_of_height: self.projection_max_fraction_of_height,
			vase_profile_epsilon: self.vase_profile_epsilon,
			projection_center_fraction: self.projection_center_fraction,
			max_depth_first_ring: self.max_depth_first_ring,
			max_depth_last_ring: self.max_depth_last_ring,
		}
	}

	/// Canopy [`SopesBanyanHysteresis`] seeds using this struct’s shared [`NoiseParams`].
	pub fn hysteresis_seeds(&self) -> Vec<SopesBanyanHysteresis> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors()
			.hysteresis_seeds(noise, self.banyan_height, self.descender_threshold)
	}
}
