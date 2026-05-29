//! CLI wrappers that flatten tuft shape args and skipped render materials.

use bevy::prelude::*;
use chico_ball_components::tuft::{
	BladeTuftShape, SucculentTuftShape, WeepingTuftShape,
};
use chico_sbs_trees::SkippedLeafMeshMaterial;

use crate::render::{RenderBladeTuft, RenderSucculentTuft, RenderWeepingTuft};

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct SucculentTuftRenderParams {
	#[command(flatten)]
	pub shape: SucculentTuftShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for SucculentTuftRenderParams {
	fn default() -> Self {
		Self {
			shape: SucculentTuftShape::default(),
			material: SkippedLeafMeshMaterial::default(),
		}
	}
}

impl From<SucculentTuftRenderParams> for RenderSucculentTuft {
	fn from(params: SucculentTuftRenderParams) -> Self {
		RenderSucculentTuft::from_shape(params.shape, params.material)
	}
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct BladeTuftRenderParams {
	#[command(flatten)]
	pub shape: BladeTuftShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for BladeTuftRenderParams {
	fn default() -> Self {
		Self {
			shape: BladeTuftShape::default(),
			material: SkippedLeafMeshMaterial::default(),
		}
	}
}

impl From<BladeTuftRenderParams> for RenderBladeTuft {
	fn from(params: BladeTuftRenderParams) -> Self {
		RenderBladeTuft::from_shape(params.shape, params.material)
	}
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct WeepingTuftRenderParams {
	#[command(flatten)]
	pub shape: WeepingTuftShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for WeepingTuftRenderParams {
	fn default() -> Self {
		Self {
			shape: WeepingTuftShape::default(),
			material: SkippedLeafMeshMaterial::default(),
		}
	}
}

impl From<WeepingTuftRenderParams> for RenderWeepingTuft {
	fn from(params: WeepingTuftRenderParams) -> Self {
		RenderWeepingTuft::from_shape(params.shape, params.material)
	}
}
