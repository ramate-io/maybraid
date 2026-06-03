pub mod plugin;

use crate::render::RenderKamakuraTorch;

use super::RenderHelper;

/// Renders Kamakura Torch into the scene.
pub type KamakuraTorchRenderHelper = RenderHelper<RenderKamakuraTorch>;
