//! CLI wrappers for moderate-LOD frond crown shape args and skipped render materials.

use bevy::prelude::*;
use chico_ball_components::ModerateLodFrondCrownShape;
use chico_sbs_trees::SkippedLeafMeshMaterial;

use crate::render::RenderModerateLodFrondCrown;

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct ModerateLodFrondCrownRenderParams {
	#[command(flatten)]
	pub shape: ModerateLodFrondCrownShape,
	#[command(flatten, next_help_heading = "Material")]
	pub material: SkippedLeafMeshMaterial<StandardMaterial>,
}

impl Default for ModerateLodFrondCrownRenderParams {
	fn default() -> Self {
		Self {
			shape: ModerateLodFrondCrownShape::default(),
			material: SkippedLeafMeshMaterial::default(),
		}
	}
}

impl From<ModerateLodFrondCrownRenderParams> for RenderModerateLodFrondCrown {
	fn from(params: ModerateLodFrondCrownRenderParams) -> Self {
		RenderModerateLodFrondCrown::from_shape(params.shape, params.material)
	}
}
