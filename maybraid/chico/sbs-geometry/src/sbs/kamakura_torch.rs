//! Restricted **Kamakura Torch** geometry for CLI and playgrounds (stashed near-vertical flame).

use bevy_math::Vec3;
#[cfg(feature = "clap")]
use procedural_common::{noise_params_from_scalar_str, parse_unit_range};
use procedural_common::{NoiseConfig, NoiseParams, SetNoiseParams, UnitRange};

use crate::anchors::kamakura_torch::{
	KamakuraTorchAnchorPerturbation, KamakuraTorchAnchors, KamakuraTorchProtoAnchors,
	DEFAULT_FIRST_RING_UNIT_HEIGHT, DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
	DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT, DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
	DEFAULT_STALK_BASE_RADIUS_FRACTION, DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_TREE_HEIGHT,
	DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
	DEFAULT_TORCH_BIAS_HIGH_DEGREES, DEFAULT_TORCH_BIAS_LOW_DEGREES,
	DEFAULT_VASE_PROFILE_EPSILON,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::BallStickChain;
use crate::StorybookTreeChain;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TREE_HEIGHT))]
	pub tree_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT_FRACTION))]
	pub stalk_height_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
	#[cfg_attr(
		feature = "clap",
		arg(
			long,
			default_value = "0,0,0",
			value_parser = crate::vec3_args::parse_vec3_csv,
			value_name = "X,Y,Z"
		)
	)]
	pub base_anchor: Vec3,
}

impl Default for KamakuraTorchScale {
	fn default() -> Self {
		Self {
			tree_height: DEFAULT_TREE_HEIGHT,
			stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_base_radius: None,
			base_anchor: Vec3::ZERO,
		}
	}
}

impl KamakuraTorchScale {
	pub fn stalk_height(&self) -> f32 {
		self.tree_height.max(1e-6) * self.stalk_height_fraction
	}

	pub fn stalk_base_radius_or_default(&self) -> f32 {
		self.stalk_base_radius
			.unwrap_or(DEFAULT_STALK_BASE_RADIUS_FRACTION * self.tree_height)
	}

	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk {
			stalk_height: self.stalk_height(),
			stalk_base_anchor: self.base_anchor,
			stalk_base_radius: self.stalk_base_radius_or_default(),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchRingParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.20..1.0",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value = "0.1142857"))]
	pub spacing: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub anchors_per_ring: u32,
}

impl Default for KamakuraTorchRingParams {
	fn default() -> Self {
		Self {
			height_range: UnitRange::new(DEFAULT_FIRST_RING_UNIT_HEIGHT, 1.0),
			spacing: 0.08 / DEFAULT_STALK_HEIGHT_FRACTION,
			anchors_per_ring: 6,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchProjectionParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.10..0.45",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub span_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_VASE_PROFILE_EPSILON))]
	pub vase_profile_epsilon: f32,
}

impl KamakuraTorchProjectionParams {
	pub fn min_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.min(self.span_fraction_of_height.end)
	}

	pub fn max_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.max(self.span_fraction_of_height.end)
	}
}

impl Default for KamakuraTorchProjectionParams {
	fn default() -> Self {
		Self {
			span_fraction_of_height: UnitRange::new(
				DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
				DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			),
			vase_profile_epsilon: DEFAULT_VASE_PROFILE_EPSILON,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchGrowthParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES))]
	pub angle_tolerance_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TORCH_BIAS_LOW_DEGREES))]
	pub torch_bias_low_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TORCH_BIAS_HIGH_DEGREES))]
	pub torch_bias_high_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1))]
	pub child_count_min: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3))]
	pub child_count_max: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.12))]
	pub branch_base_radius_fraction_of_stalk: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.75))]
	pub branch_radius_child_scale_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.82))]
	pub branch_radius_child_scale_hi: f32,
}

impl Default for KamakuraTorchGrowthParams {
	fn default() -> Self {
		Self {
			branch_depth: 4,
			angle_tolerance_degrees: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
			torch_bias_low_degrees: DEFAULT_TORCH_BIAS_LOW_DEGREES,
			torch_bias_high_degrees: DEFAULT_TORCH_BIAS_HIGH_DEGREES,
			child_count_min: 1,
			child_count_max: 3,
			branch_base_radius_fraction_of_stalk: 0.12,
			branch_radius_child_scale_lo: 0.75,
			branch_radius_child_scale_hi: 0.82,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchCanopyParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.06))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION)
	)]
	pub outer_foliage_distance_fraction: f32,
}

impl Default for KamakuraTorchCanopyParams {
	fn default() -> Self {
		Self {
			leaf_radius_fraction: 0.06,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchAnchorPerturbationParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-vertical-perturbation",
			default_value = "-1.0..1.0",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub vertical_offset: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-angular-perturbation",
			default_value = "0.0..0.5",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub angular_scale: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-radius-perturbation",
			default_value = "-0.05..0.05",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub radius_offset: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-perturbation-noise",
			default_value = "1337,1,1,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES"
		)
	)]
	pub noise: NoiseParams,
}

impl Default for KamakuraTorchAnchorPerturbationParams {
	fn default() -> Self {
		Self {
			vertical_offset: UnitRange::new(-1.0, 1.0),
			angular_scale: UnitRange::new(0.0, 0.5),
			radius_offset: UnitRange::new(-0.05, 0.05),
			noise: NoiseParams::default(),
		}
	}
}

impl KamakuraTorchAnchorPerturbationParams {
	pub fn to_perturbation(&self) -> KamakuraTorchAnchorPerturbation {
		KamakuraTorchAnchorPerturbation {
			noise: self.noise,
			vertical_offset: self.vertical_offset.start..self.vertical_offset.end,
			angular_scale: self.angular_scale.start..self.angular_scale.end,
			radius_offset: self.radius_offset.start..self.radius_offset.end,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct KamakuraTorchSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: KamakuraTorchScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Rings"))]
	pub rings: KamakuraTorchRingParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: KamakuraTorchProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: KamakuraTorchGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy"))]
	pub canopy: KamakuraTorchCanopyParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: KamakuraTorchAnchorPerturbationParams,
}

impl Default for KamakuraTorchSbs {
	fn default() -> Self {
		Self {
			scale: KamakuraTorchScale::default(),
			rings: KamakuraTorchRingParams::default(),
			projection: KamakuraTorchProjectionParams::default(),
			growth: KamakuraTorchGrowthParams::default(),
			canopy: KamakuraTorchCanopyParams::default(),
			canopy_noise: NoiseParams::default(),
			anchor_perturbation: KamakuraTorchAnchorPerturbationParams::default(),
		}
	}
}

impl KamakuraTorchSbs {
	pub fn height(&self) -> f32 {
		self.scale.tree_height.max(1e-6)
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.height() * self.canopy.leaf_radius_fraction
	}

	pub fn to_proto(&self) -> KamakuraTorchProtoAnchors {
		KamakuraTorchProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			projection_min_fraction_of_height: self.projection.min_fraction(),
			projection_max_fraction_of_height: self.projection.max_fraction(),
			vase_profile_epsilon: self.projection.vase_profile_epsilon,
			projection_center_fraction: 0.5,
			torch_bias_low_degrees: self.growth.torch_bias_low_degrees,
			torch_bias_high_degrees: self.growth.torch_bias_high_degrees,
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			bias_blend: 1.0,
			branch_depth: self.growth.branch_depth,
			child_count_min: self.growth.child_count_min,
			child_count_max: self.growth.child_count_max.max(self.growth.child_count_min),
			outer_foliage_distance_fraction: self.canopy.outer_foliage_distance_fraction,
			branch_base_radius_fraction_of_stalk: self.growth.branch_base_radius_fraction_of_stalk,
			branch_radius_child_scale: (
				self.growth.branch_radius_child_scale_lo,
				self.growth.branch_radius_child_scale_hi,
			),
			..KamakuraTorchProtoAnchors::default()
		}
	}

	pub fn to_anchors(&self) -> KamakuraTorchAnchors {
		KamakuraTorchAnchors::new(self.to_proto())
			.with_perturbation(self.anchor_perturbation.to_perturbation())
	}

	pub fn hysteresis_seeds(&self) -> Vec<StorybookTreeChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for KamakuraTorchSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds()
	}
}

impl SetNoiseParams for KamakuraTorchSbs {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		self.canopy_noise = params;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_frontend_converts_to_proto() -> Result<()> {
		let sbs = KamakuraTorchSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.tree_height - DEFAULT_TREE_HEIGHT).abs() < 1e-5);
		assert!(
			(proto.stalk.stalk_height - DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION).abs()
				< 1e-4
		);
		assert_eq!(proto.branch_depth, 4);
		assert_eq!(proto.anchors_per_ring, 6);
		assert!(
			(proto.projection_max_fraction_of_height - DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT)
				.abs()
				< 1e-5
		);
		Ok(())
	}

	#[test]
	fn build_chain_produces_substantial_graph() {
		let chain = KamakuraTorchSbs::default().build_chain();
		assert!(chain.nodes.len() > 20, "nodes {}", chain.nodes.len());
	}
}
