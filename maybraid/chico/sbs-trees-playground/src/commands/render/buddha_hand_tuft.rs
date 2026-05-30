pub mod plugin;

use crate::tuft_render_params::BuddhaHandTuftRenderParams;

use super::RenderHelper;

/// Renders a standalone Buddha's-hand tuft into the scene.
pub type BuddhaHandTuftRenderHelper = RenderHelper<BuddhaHandTuftRenderParams>;
