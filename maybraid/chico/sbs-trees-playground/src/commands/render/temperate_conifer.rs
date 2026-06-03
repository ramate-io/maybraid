pub mod plugin;

use crate::render::RenderTemperateConifer;

use super::RenderHelper;

/// Renders Temperate Conifer into the scene.
pub type TemperateConiferRenderHelper = RenderHelper<RenderTemperateConifer>;
