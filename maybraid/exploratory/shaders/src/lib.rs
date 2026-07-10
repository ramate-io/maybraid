//! Exploratory Bevy materials and embedded WGSL for stylized rendering experiments.

mod eyeball_shader;
mod splatter_shader;
mod watercolor_post_process;
mod watercolor_shader;

pub use eyeball_shader::{EyeballAlbedo, EyeballShader, EyeballShaderPlugin};
pub use splatter_shader::{SplatterAlbedo, SplatterShader, SplatterShaderPlugin};
pub use watercolor_post_process::{WatercolorPostProcess, WatercolorPostProcessPlugin};
pub use watercolor_shader::{
	WatercolorLightingUniform, WatercolorPaperUniform, WatercolorShader, WatercolorShaderPlugin,
	WatercolorShadowUniform,
};
