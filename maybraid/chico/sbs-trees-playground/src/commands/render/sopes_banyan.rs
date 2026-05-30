pub mod plugin;

use crate::render::RenderSopesBanyan;

use super::RenderHelper;

/// Renders the current Sope's Banyan assembly into the scene.
pub type SopesBanyanRenderHelper = RenderHelper<RenderSopesBanyan>;
