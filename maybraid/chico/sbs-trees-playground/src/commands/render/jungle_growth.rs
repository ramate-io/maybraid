pub mod plugin;

use crate::jungle_growth_render_params::JungleGrowthRenderParams;

use super::RenderHelper;

/// Renders a standalone jungle-growth cluster (inner mass + drooping tuft) into the scene.
pub type JungleGrowthRenderHelper = RenderHelper<JungleGrowthRenderParams>;
