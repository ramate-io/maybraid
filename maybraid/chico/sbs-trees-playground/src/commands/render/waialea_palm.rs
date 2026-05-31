pub mod plugin;

use crate::render::RenderWaialeaPalm;

use super::RenderHelper;

/// Renders Waialea Palm into the scene.
pub type WaialeaPalmRenderHelper = RenderHelper<RenderWaialeaPalm>;
