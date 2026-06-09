//! CLI wrappers that flatten tuft shape args and skipped render materials.

use bevy::prelude::*;
use chico_ball_components::tuft::{
	BladeTuftShape, BuddhaHandTuftShape, SpearTuftShape, SucculentTuftShape, WeepingTuftShape,
};
use chico_sbs_trees::SkippedLeafMeshMaterial;

use crate::render::{
	RenderBladeTuft, RenderBuddhaHandTuft, RenderSpearTuft, RenderSucculentTuft, RenderWeepingTuft,
};

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
		Self { shape: SucculentTuftShape::default(), material: SkippedLeafMeshMaterial::default() }
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
		Self { shape: BladeTuftShape::default(), material: SkippedLeafMeshMaterial::default() }
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
		Self { shape: WeepingTuftShape::default(), material: SkippedLeafMeshMaterial::default() }
	}
}

impl From<WeepingTuftRenderParams> for RenderWeepingTuft {
	fn from(params: WeepingTuftRenderParams) -> Self {
		RenderWeepingTuft::from_shape(params.shape, params.material)
	}
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct SpearTuftRenderParams {
	#[command(flatten)]
	pub shape: SpearTuftShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for SpearTuftRenderParams {
	fn default() -> Self {
		Self { shape: SpearTuftShape::default(), material: SkippedLeafMeshMaterial::default() }
	}
}

impl From<SpearTuftRenderParams> for RenderSpearTuft {
	fn from(params: SpearTuftRenderParams) -> Self {
		RenderSpearTuft::from_shape(params.shape, params.material)
	}
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct BuddhaHandTuftRenderParams {
	#[command(flatten)]
	pub shape: BuddhaHandTuftShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for BuddhaHandTuftRenderParams {
	fn default() -> Self {
		Self { shape: BuddhaHandTuftShape::default(), material: SkippedLeafMeshMaterial::default() }
	}
}

impl From<BuddhaHandTuftRenderParams> for RenderBuddhaHandTuft {
	fn from(params: BuddhaHandTuftRenderParams) -> Self {
		RenderBuddhaHandTuft::from_shape(params.shape, params.material)
	}
}
