pub mod plugin;

use crate::render::RenderBraidOakTree;

use super::RenderHelper;

/// Renders Braid Oak Tree into the scene ([#234](https://github.com/ramate-io/maybraid/issues/234)).
pub type BraidOakTreeRenderHelper = RenderHelper<RenderBraidOakTree>;
