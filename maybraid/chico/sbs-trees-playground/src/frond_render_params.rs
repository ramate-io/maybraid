//! CLI wrappers for frond crown shape args and skipped render materials.

use bevy::prelude::*;
use chico_ball_components::FrondCrownShape;
use chico_sbs_trees::SkippedLeafMeshMaterial;

use crate::render::RenderFrondCrown;

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct FrondCrownRenderParams {
	#[command(flatten)]
	pub shape: FrondCrownShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for FrondCrownRenderParams {
	fn default() -> Self {
		Self {
			shape: FrondCrownShape::default(),
			material: SkippedLeafMeshMaterial::default(),
		}
	}
}

impl From<FrondCrownRenderParams> for RenderFrondCrown {
	fn from(params: FrondCrownRenderParams) -> Self {
		RenderFrondCrown::from_shape(params.shape, params.material)
	}
}
