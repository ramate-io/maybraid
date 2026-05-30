pub mod plugin;

use crate::tuft_render_params::SpearTuftRenderParams;

use super::RenderHelper;

/// Renders a standalone spear tuft into the scene.
pub type SpearTuftRenderHelper = RenderHelper<SpearTuftRenderParams>;
