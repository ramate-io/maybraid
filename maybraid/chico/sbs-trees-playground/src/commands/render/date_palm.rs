pub mod plugin;

use crate::render::RenderDatePalm;

use super::RenderHelper;

/// Renders Date Palm into the scene.
pub type DatePalmRenderHelper = RenderHelper<RenderDatePalm>;
