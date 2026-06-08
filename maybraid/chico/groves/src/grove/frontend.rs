//! Shared grove assembly CLI surface reused by well-known grove render items.

use bevy_math::Vec3;

use super::{
	parse_variant_weights, CellGrove, ForestGroveBiases, Grove, GroveNoiseConfig,
	VariantWeightOverrides,
};

#[cfg(feature = "render")]
use super::parse_vec3_csv;

/// Forest biases, shared noise, distribution overrides, and perturbation origin.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(rename_all = "kebab-case"))]
pub struct GroveFrontend {
	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove Biases"))]
	pub biases: ForestGroveBiases,

	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove Noise"))]
	pub noise: GroveNoiseConfig,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			value_parser = parse_variant_weights,
			value_name = "W0,W1,...",
			help_heading = "Grove Distribution",
			help = "Per-bucket weights in distribution order; x keeps the authored default"
		)
	)]
	pub variant_weights: Option<VariantWeightOverrides>,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			default_value = "0,0,0",
			value_parser = parse_vec3_csv,
			value_name = "X,Y,Z",
			help_heading = "Grove Assembly",
		)
	)]
	pub perturbation_origin: Vec3,
}

impl Default for GroveFrontend {
	fn default() -> Self {
		Self {
			biases: ForestGroveBiases::default(),
			noise: GroveNoiseConfig::default(),
			variant_weights: None,
			perturbation_origin: Vec3::ZERO,
		}
	}
}

impl GroveFrontend {
	pub fn assemble<G: CellGrove>(&self, definition: G) -> Grove<G>
	where
		G::Variant: Clone,
	{
		Grove::assemble(
			definition,
			self.biases,
			self.noise,
			self.perturbation_origin,
		)
	}
}
