pub mod plugin;

use crate::render::RenderHonuBanyan;

use super::RenderHelper;

/// Renders the current Honu Banyan assembly into the scene.
pub type HonuBanyanRenderHelper = RenderHelper<RenderHonuBanyan>;
