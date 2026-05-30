pub mod plugin;

use crate::tuft_render_params::SucculentTuftRenderParams;

use super::RenderHelper;

/// Renders a standalone succulent tuft into the scene.
pub type SucculentTuftRenderHelper = RenderHelper<SucculentTuftRenderParams>;
