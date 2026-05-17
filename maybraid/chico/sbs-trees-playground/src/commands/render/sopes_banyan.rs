pub mod plugin;

use crate::preview::PreviewSopesBanyan;

use super::RenderHelper;

/// Renders the current Sope's Banyan assembly into the preview scene.
pub type SopesBanyanRenderHelper = RenderHelper<PreviewSopesBanyan>;
