pub mod plugin;

use crate::render::RenderNorthernConifer;

use super::RenderHelper;

/// Renders Northern Conifer into the scene.
pub type NorthernConiferRenderHelper = RenderHelper<RenderNorthernConifer>;
