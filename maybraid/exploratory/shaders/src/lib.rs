//! Exploratory Bevy materials and embedded WGSL for stylized rendering experiments.

mod watercolor_post_process;
mod watercolor_shader;

pub use watercolor_post_process::{WatercolorPostProcess, WatercolorPostProcessPlugin};
pub use watercolor_shader::{
	WatercolorLightingUniform, WatercolorPaperUniform, WatercolorShadowUniform, WatercolorShader,
	WatercolorShaderPlugin,
};
