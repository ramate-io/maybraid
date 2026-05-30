//! CLI wrapper for [`JungleGrowth`] shape args and skipped render materials.

use bevy::prelude::*;
use chico_tree_components::{
	JungleGrowth, JungleGrowthShape, SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
};
use chico_vegetation_shaders::ChicoStickMaterial;

use crate::render::RenderJungleGrowth;

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct JungleGrowthRenderParams {
	#[command(flatten)]
	pub shape: JungleGrowthShape,
	#[command(flatten, next_help_heading = "Body Material")]
	pub body_material: SkippedBodyMeshMaterial<ChicoStickMaterial>,
	#[command(flatten, next_help_heading = "Foliage Material")]
	pub foliage_material: SkippedFoliageMeshMaterial<StandardMaterial>,
}

impl Default for JungleGrowthRenderParams {
	fn default() -> Self {
		Self {
			shape: JungleGrowthShape::default(),
			body_material: SkippedBodyMeshMaterial::default(),
			foliage_material: SkippedFoliageMeshMaterial::default(),
		}
	}
}

impl From<JungleGrowthRenderParams> for RenderJungleGrowth {
	fn from(params: JungleGrowthRenderParams) -> Self {
		let mut growth = JungleGrowth::<
			ChicoStickMaterial,
			SkippedBodyMeshMaterial<ChicoStickMaterial>,
			StandardMaterial,
			SkippedFoliageMeshMaterial<StandardMaterial>,
		>::default();
		growth.shape = params.shape;
		growth.body_material = params.body_material;
		growth.foliage_material = params.foliage_material;
		growth
	}
}
