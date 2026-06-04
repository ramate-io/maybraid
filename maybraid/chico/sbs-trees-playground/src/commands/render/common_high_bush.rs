pub mod plugin;

use crate::high_bush_shoots_render_params::CommonHighBushRenderParams;

use super::RenderHelper;

/// Common High Bush preset preview ([#233](https://github.com/ramate-io/maybraid/issues/233)).
pub type CommonHighBushRenderHelper = RenderHelper<CommonHighBushRenderParams>;
