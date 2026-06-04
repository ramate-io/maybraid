pub mod plugin;

use crate::high_bush_shoots_render_params::HighBushShootsRenderParams;

use super::RenderHelper;

/// Renders a standalone high-bush radial shoot construction ([#225](https://github.com/ramate-io/maybraid/issues/225)).
pub type HighBushShootsRenderHelper = RenderHelper<HighBushShootsRenderParams>;
