pub mod plugin;

use chico_sbs_trees::sopes_banyan::SopesBanyan;

use super::RenderHelper;

/// Renders the current Sope's Banyan assembly into the preview scene.
pub type SopesBanyanRenderHelper = RenderHelper<SopesBanyan>;
