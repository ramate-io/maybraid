pub mod plugin;

use crate::render::RenderJungleStorybookTree;

use super::RenderHelper;

/// Renders Jungle Storybook Tree into the scene ([#235](https://github.com/ramate-io/maybraid/issues/235)).
pub type JungleStorybookTreeRenderHelper = RenderHelper<RenderJungleStorybookTree>;
