//! CLI wrapper for [`HighBushShoots`] shape args and skipped render materials.

use bevy::prelude::StandardMaterial;
use chico_sbs_trees::SkippedLeafMeshMaterial;
use chico_sbs_trees::SkippedStickMeshMaterial;
use chico_tree_components::{apply_common_high_bush_preset, HighBushShootsShape};
use chico_vegetation_shaders::ChicoStickMaterial;
use procedural_common::{FromScalarNoise, NoiseParams};

use crate::render::RenderHighBushShoots;

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct HighBushShootsRenderParams {
	#[command(flatten)]
	pub shape: HighBushShootsShape,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: SkippedStickMeshMaterial<ChicoStickMaterial>,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: SkippedLeafMeshMaterial<StandardMaterial>,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = procedural_common::noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Stick Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = procedural_common::noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Foliage Surface Noise"
	)]
	pub leaf_surface_noise: NoiseParams,
}

impl Default for HighBushShootsRenderParams {
	fn default() -> Self {
		Self {
			shape: HighBushShootsShape::default(),
			stick_material: SkippedStickMeshMaterial::default(),
			leaf_material: SkippedLeafMeshMaterial::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
		}
	}
}

impl From<HighBushShootsRenderParams> for RenderHighBushShoots {
	fn from(params: HighBushShootsRenderParams) -> Self {
		let mut shape = params.shape;
		apply_common_high_bush_preset(&mut shape);
		let mut shoots = Self::default();
		shoots.shape = shape;
		shoots.stick_surface_noise = params.stick_surface_noise;
		shoots.leaf_surface_noise = params.leaf_surface_noise;
		shoots.stick_material = params.stick_material;
		shoots.leaf_material = params.leaf_material;
		shoots
	}
}

/// Common High Bush preset wrapper ([#233](https://github.com/ramate-io/maybraid/issues/233)).
#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct CommonHighBushRenderParams {
	#[command(flatten)]
	pub inner: HighBushShootsRenderParams,
}

impl From<CommonHighBushRenderParams> for RenderHighBushShoots {
	fn from(params: CommonHighBushRenderParams) -> Self {
		params.inner.into()
	}
}
