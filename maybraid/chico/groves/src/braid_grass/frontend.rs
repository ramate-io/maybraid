//! Braid Grass CLI grove surface — defaults defer to [`BraidGrassDefinition`] authored weights.

use bevy_math::Vec3;

use super::BraidGrassDefinition;
use crate::grove::{
	parse_variant_weights, ForestGroveBiases, GroveFrontend, GroveNoiseConfig,
	VariantWeightOverrides,
};

#[cfg(feature = "render")]
use crate::grove::parse_vec3_csv;

/// [`GroveFrontend`] for Braid Grass; omits `--variant-weights` unless the caller overrides.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(rename_all = "kebab-case"))]
pub struct BraidGrassGroveFrontend {
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
			help = "Per-bucket weights in distribution order; x keeps the authored default. \
			        Omitted flag uses authored braid-grass definition weights \
			        (BraidGrassDefinition::VARIANT_WEIGHTS_CLI)."
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

#[cfg(feature = "render")]
fn braid_grass_variant_weights_default() -> Option<VariantWeightOverrides> {
	None
}

impl Default for BraidGrassGroveFrontend {
	fn default() -> Self {
		Self {
			biases: ForestGroveBiases::default(),
			noise: GroveNoiseConfig::default(),
			variant_weights: braid_grass_variant_weights_default(),
			perturbation_origin: Vec3::ZERO,
		}
	}
}

impl From<BraidGrassGroveFrontend> for GroveFrontend {
	fn from(frontend: BraidGrassGroveFrontend) -> Self {
		Self {
			biases: frontend.biases,
			noise: frontend.noise,
			variant_weights: frontend.variant_weights,
			perturbation_origin: frontend.perturbation_origin,
		}
	}
}

impl BraidGrassGroveFrontend {
	pub fn assemble(&self, definition: BraidGrassDefinition) -> crate::grove::Grove<BraidGrassDefinition> {
		GroveFrontend::from(self.clone()).assemble(definition)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::CellGrove;
	use crate::BraidGrassCell;
	use anyhow::Result;

	#[test]
	fn default_variant_weights_defer_to_definition() -> Result<()> {
		let frontend = BraidGrassGroveFrontend::default();
		assert!(frontend.variant_weights.is_none());
		let definition = BraidGrassDefinition::new();
		let authored = BraidGrassCell::grove_distribution();
		for (bucket, authored_bucket) in definition
			.distribution()
			.buckets
			.iter()
			.zip(&authored.buckets)
		{
			assert_eq!(bucket.weight, authored_bucket.weight);
		}
		Ok(())
	}

	#[test]
	fn variant_weights_cli_matches_distribution() -> Result<()> {
		let authored = BraidGrassCell::grove_distribution();
		let expected: Vec<String> = authored.buckets.iter().map(|b| b.weight.to_string()).collect();
		let cli = BraidGrassDefinition::VARIANT_WEIGHTS_CLI
			.split(',')
			.map(str::trim)
			.map(str::to_string)
			.collect::<Vec<_>>();
		assert_eq!(cli, expected);
		Ok(())
	}
}
