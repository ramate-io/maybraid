//! Tropical Tufts CLI grove surface — defaults defer to [`TropicalTuftsDefinition`] authored weights.

use bevy_math::{Vec2, Vec3};

use super::TropicalTuftsDefinition;
use crate::grove::{
	parse_variant_weights, ForestGroveBiases, GroveFrontend, GroveNoiseConfig,
	VariantWeightOverrides,
};

#[cfg(feature = "render")]
use crate::grove::{parse_vec2_csv, parse_vec3_csv};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(rename_all = "kebab-case"))]
pub struct TropicalTuftsGroveFrontend {
	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove Biases"))]
	pub biases: ForestGroveBiases,

	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove Noise"))]
	pub noise: GroveNoiseConfig,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			default_value = "3.25,3.25",
			value_parser = parse_vec2_csv,
			value_name = "X,Z",
			help_heading = "Grove Assembly",
		)
	)]
	pub cell_extent_xz: Vec2,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			value_parser = parse_variant_weights,
			value_name = "W0,W1,...",
			help_heading = "Grove Distribution",
			help = "Per-bucket weights in distribution order; x keeps the authored default. \
			        Omitted flag uses authored tropical-tufts definition weights \
			        (TropicalTuftsDefinition::VARIANT_WEIGHTS_CLI)."
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

impl Default for TropicalTuftsGroveFrontend {
	fn default() -> Self {
		Self {
			biases: ForestGroveBiases::default(),
			noise: GroveNoiseConfig::default(),
			cell_extent_xz: TropicalTuftsDefinition::cell_extent_xz_default(),
			variant_weights: None,
			perturbation_origin: Vec3::ZERO,
		}
	}
}

impl From<TropicalTuftsGroveFrontend> for GroveFrontend {
	fn from(frontend: TropicalTuftsGroveFrontend) -> Self {
		Self {
			biases: frontend.biases,
			noise: frontend.noise,
			variant_weights: frontend.variant_weights,
			perturbation_origin: frontend.perturbation_origin,
		}
	}
}

impl TropicalTuftsGroveFrontend {
	pub fn assemble(
		&self,
		definition: TropicalTuftsDefinition,
	) -> crate::grove::Grove<TropicalTuftsDefinition> {
		GroveFrontend::from(self.clone())
			.assemble(definition.with_cell_extent_xz(self.cell_extent_xz))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::CellGrove;
	use crate::TropicalTuftsCell;
	use anyhow::Result;

	#[test]
	fn default_variant_weights_defer_to_definition() -> Result<()> {
		let frontend = TropicalTuftsGroveFrontend::default();
		assert!(frontend.variant_weights.is_none());
		let definition = TropicalTuftsDefinition::new();
		let authored = TropicalTuftsCell::grove_distribution();
		for (bucket, authored_bucket) in
			definition.distribution().buckets.iter().zip(&authored.buckets)
		{
			assert_eq!(bucket.weight, authored_bucket.weight);
		}
		Ok(())
	}

	#[test]
	fn variant_weights_cli_matches_distribution() -> Result<()> {
		let authored = TropicalTuftsCell::grove_distribution();
		let expected: Vec<String> = authored.buckets.iter().map(|b| b.weight.to_string()).collect();
		let cli = TropicalTuftsDefinition::VARIANT_WEIGHTS_CLI
			.split(',')
			.map(str::trim)
			.map(str::to_string)
			.collect::<Vec<_>>();
		assert_eq!(cli, expected);
		Ok(())
	}

	#[test]
	fn assemble_applies_frontend_cell_extent() -> Result<()> {
		let frontend =
			TropicalTuftsGroveFrontend { cell_extent_xz: Vec2::new(4.0, 3.0), ..Default::default() };
		let grove = frontend.assemble(TropicalTuftsDefinition::new());
		assert_eq!(grove.cell_extent_xz(), Vec2::new(4.0, 3.0));
		Ok(())
	}
}
