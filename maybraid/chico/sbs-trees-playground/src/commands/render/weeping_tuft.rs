pub mod plugin;

use crate::tuft_render_params::WeepingTuftRenderParams;

use super::RenderHelper;

/// Renders a standalone weeping tuft into the scene.
pub type WeepingTuftRenderHelper = RenderHelper<WeepingTuftRenderParams>;
