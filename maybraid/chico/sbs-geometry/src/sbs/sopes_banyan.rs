//! Restricted **Sope's Banyan** geometry for CLI and playgrounds.

use bevy_math::Vec3;
#[cfg(any(feature = "clap", test))]
use procedural_common::{
	parse_count_pair as parse_ring_layout, parse_unit_range, parse_usize_range as parse_depth_range,
};
use procedural_common::{
	CountPair as RingLayout, NoiseConfig, NoiseParams, SetNoiseParams, UnitRange,
	UsizeRange as DepthRange,
};

use crate::anchors::sopes_banyan::SopesBanyanAnchors;
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::{BallStickChain, SopesBanyanChain};

/// High-level world scale for the art-directed Sope's Banyan recipe.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 20.0))]
	pub stalk_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 40.0))]
	pub canopy_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.75))]
	pub stalk_base_radius: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value = "0,0,0", value_parser = crate::vec3_args::parse_vec3_csv)
	)]
	pub base_anchor: Vec3,
}

impl Default for SopesBanyanScale {
	fn default() -> Self {
		Self {
			stalk_height: 20.0,
			canopy_height: 40.0,
			stalk_base_radius: 0.75,
			base_anchor: Vec3::ZERO,
		}
	}
}

impl SopesBanyanScale {
	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk {
			stalk_height: self.stalk_height,
			stalk_base_anchor: self.base_anchor,
			stalk_base_radius: self.stalk_base_radius,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct RingAnchorParams {
	#[cfg_attr(
		feature = "clap",
		arg(long = "rings", default_value = "8x7", value_parser = parse_ring_layout)
	)]
	pub layout: RingLayout,
	#[cfg_attr(
		feature = "clap",
		arg(long = "ring-heights", default_value = "0.40..0.95", value_parser = parse_unit_range)
	)]
	pub height_range: UnitRange,
}

impl Default for RingAnchorParams {
	fn default() -> Self {
		Self { layout: RingLayout::new(8, 7), height_range: UnitRange::new(0.40, 0.95) }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseProjectionParams {
	#[cfg_attr(
		feature = "clap",
		arg(long = "projection", default_value = "0.10..0.20", value_parser = parse_unit_range)
	)]
	pub length_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.4))]
	pub profile_epsilon: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.5))]
	pub center_fraction: f32,
}

impl Default for VaseProjectionParams {
	fn default() -> Self {
		Self {
			length_fraction_of_height: UnitRange::new(0.10, 0.20),
			profile_epsilon: 0.4,
			center_fraction: 0.5,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct CanopyGrowthParams {
	#[cfg_attr(
		feature = "clap",
		arg(long = "depth", default_value = "4..8", value_parser = parse_depth_range)
	)]
	pub depth: DepthRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.01))]
	pub descender_threshold: f32,
}

impl Default for CanopyGrowthParams {
	fn default() -> Self {
		Self { depth: DepthRange::new(4, 8), descender_threshold: 0.01 }
	}
}

/// Art-directed front-end: scale + rings + projection + growth + one structural canopy noise.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub scale: SopesBanyanScale,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub rings: RingAnchorParams,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub projection: VaseProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub growth: CanopyGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub canopy_noise: NoiseParams,
}

impl Default for SopesBanyanSbs {
	fn default() -> Self {
		Self {
			scale: SopesBanyanScale::default(),
			rings: RingAnchorParams::default(),
			projection: VaseProjectionParams::default(),
			growth: CanopyGrowthParams::default(),
			canopy_noise: NoiseParams::default(),
		}
	}
}

impl SopesBanyanSbs {
	/// Full anchor recipe (stalk + rings); chain noise is applied when emitting seeds.
	pub fn to_anchors(&self) -> SopesBanyanAnchors {
		SopesBanyanAnchors {
			stalk: self.scale.to_stalk(),
			descender_threshold: self.growth.descender_threshold,
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_count: self.rings.layout.first,
			anchors_per_ring: self.rings.layout.second,
			projection_min_fraction_of_height: self.projection.length_fraction_of_height.start,
			projection_max_fraction_of_height: self.projection.length_fraction_of_height.end,
			vase_profile_epsilon: self.projection.profile_epsilon,
			projection_center_fraction: self.projection.center_fraction,
			max_depth_first_ring: self.growth.depth.start,
			max_depth_last_ring: self.growth.depth.end,
		}
	}

	/// Canopy [`SopesBanyanChain`] seeds using this struct's shared [`NoiseParams`].
	pub fn hysteresis_seeds(&self) -> Vec<SopesBanyanChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(
			noise,
			self.scale.canopy_height,
			self.growth.descender_threshold,
		)
	}

	pub fn build_chain(&self) -> BallStickChain<SopesBanyanChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<SopesBanyanChain> for SopesBanyanSbs {
	fn anchors(&self) -> Vec<SopesBanyanChain> {
		self.hysteresis_seeds()
	}
}

impl SetNoiseParams for SopesBanyanSbs {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		self.canopy_noise = params;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compact_parsers_accept_expected_shapes() -> anyhow::Result<()> {
		assert_eq!(
			parse_ring_layout("8x7").map_err(|e| anyhow::anyhow!("{e}"))?,
			RingLayout::new(8, 7)
		);
		assert_eq!(
			parse_unit_range("0.40..0.95").map_err(|e| anyhow::anyhow!("{e}"))?,
			UnitRange::new(0.40, 0.95)
		);
		assert_eq!(
			parse_depth_range("4..8").map_err(|e| anyhow::anyhow!("{e}"))?,
			DepthRange::new(4, 8)
		);
		Ok(())
	}

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = SopesBanyanSbs::default();
		let anchors = sbs.to_anchors();

		assert_eq!(anchors.stalk, sbs.scale.to_stalk());
		assert_eq!(anchors.ring_count, sbs.rings.layout.first);
		assert_eq!(anchors.anchors_per_ring, sbs.rings.layout.second);
		assert_eq!(anchors.first_ring_unit_height, sbs.rings.height_range.start);
		assert_eq!(anchors.last_ring_unit_height, sbs.rings.height_range.end);
		assert_eq!(
			anchors.projection_min_fraction_of_height,
			sbs.projection.length_fraction_of_height.start
		);
		assert_eq!(
			anchors.projection_max_fraction_of_height,
			sbs.projection.length_fraction_of_height.end
		);
		assert_eq!(anchors.max_depth_first_ring, sbs.growth.depth.start);
		assert_eq!(anchors.max_depth_last_ring, sbs.growth.depth.end);
	}

	#[test]
	fn scale_controls_stalk_and_canopy_height() {
		let sbs = SopesBanyanSbs {
			scale: SopesBanyanScale {
				stalk_height: 12.0,
				canopy_height: 30.0,
				..Default::default()
			},
			..Default::default()
		};
		let seeds = sbs.hysteresis_seeds();

		assert_eq!(sbs.to_anchors().stalk.stalk_height, 12.0);
		assert!(seeds.iter().all(|seed| seed.banyan_height == 30.0));
	}

	#[test]
	fn noise_params_override_frontend_noise() {
		let params = NoiseParams { seed: 99, ..Default::default() };
		let sbs = SopesBanyanSbs::default().with_noise_params(params);
		assert_eq!(sbs.canopy_noise.seed, 99);
	}
}
