//! Shape parameters for [`super::HighBushShoots`](super::assembly::HighBushShoots).

use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_BIAS_BLEND, DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
};
use chico_sbs_geometry::{high_bush_branch_depth, HighBushProtoAnchors};
use procedural_common::{NoiseConfig, NoiseParams};

use super::preset::common_high_bush_shape;

/// Terminal foliage style for playground and composing recipes.
///
/// [`Self::PlaneSplay`] / [`Self::Tuft`] drive the legacy RenderItem path
/// ([`super::HighBushShoots`](super::assembly::HighBushShoots)).
/// [`Self::CheapBall`] is the VegetationComponents default (sbs-trees); RenderItem
/// maps it to the plane-splay canopy for now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum HighBushFoliageStyle {
	#[default]
	PlaneSplay,
	Tuft,
	/// Cheap-ball terminals (VegetationComponents); RenderItem falls back to plane-splay.
	CheapBall,
}

/// Configurable high-bush construction ([RFC §3.1.6.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/03-high-bushes-and-shoots/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HighBushShootsShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10.0))]
	pub height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.02))]
	pub anchor_lift_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub shoot_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.35))]
	pub radial_strength: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub vertical_bias: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub segment_length_fraction_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.16))]
	pub segment_length_fraction_hi: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.012))]
	pub segment_radius_fraction_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.025))]
	pub segment_radius_fraction_hi: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.05))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, value_enum, default_value_t = HighBushFoliageStyle::PlaneSplay))]
	pub foliage_style: HighBushFoliageStyle,
	#[cfg_attr(
		feature = "clap",
		arg(
			long,
			default_value = "0,1,1,1",
			value_parser = procedural_common::noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
		)
	)]
	pub chain_noise: NoiseParams,
}

impl HighBushShootsShape {
	pub(crate) fn default_fields() -> Self {
		common_high_bush_shape()
	}

	pub fn to_proto(&self) -> HighBushProtoAnchors {
		HighBushProtoAnchors {
			height: self.height,
			anchor_lift_fraction: self.anchor_lift_fraction,
			shoot_count: self.shoot_count,
			radial_strength: self.radial_strength,
			vertical_bias: self.vertical_bias,
			branch_depth: high_bush_branch_depth(self.branch_depth),
			child_count_min: 1,
			child_count_max: 2,
			angle_tolerance_radians: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: DEFAULT_BIAS_BLEND,
			segment_length_fraction_lo: self.segment_length_fraction_lo,
			segment_length_fraction_hi: self.segment_length_fraction_hi,
			segment_radius_fraction_lo: self.segment_radius_fraction_lo,
			segment_radius_fraction_hi: self.segment_radius_fraction_hi,
			root_radius_fraction_of_height: 0.018,
			branch_radius_child_scale: (0.72, 0.80),
		}
	}

	pub fn build_chain(
		&self,
	) -> chico_sbs_geometry::BallStickChain<chico_sbs_geometry::HighBushChain> {
		let noise = NoiseConfig::new(self.chain_noise);
		chico_sbs_geometry::BallStickChain::build(self.to_proto().hysteresis_seeds(noise))
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.height * self.leaf_radius_fraction
	}
}

impl Default for HighBushShootsShape {
	fn default() -> Self {
		Self::default_fields()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_shape_builds_chain() -> anyhow::Result<()> {
		let chain = HighBushShootsShape::default().build_chain();
		assert!(chain.nodes.len() > 20);
		Ok(())
	}
}
