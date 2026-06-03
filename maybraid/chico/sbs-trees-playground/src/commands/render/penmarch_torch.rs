pub mod plugin;

use crate::render::RenderPenmarchTorch;

use super::RenderHelper;

/// Renders Penmarch Torch into the scene.
pub type PenmarchTorchRenderHelper = RenderHelper<RenderPenmarchTorch>;
