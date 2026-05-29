pub mod plugin;

use crate::tuft_render_params::BladeTuftRenderParams;

use super::RenderHelper;

/// Renders a standalone blade tuft into the scene.
pub type BladeTuftRenderHelper = RenderHelper<BladeTuftRenderParams>;
