pub mod plugin;

use crate::render::RenderStorybookTree;

use super::RenderHelper;

/// Renders Storybook Tree into the scene.
pub type StorybookTreeRenderHelper = RenderHelper<RenderStorybookTree>;
