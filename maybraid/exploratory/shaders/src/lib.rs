//! Exploratory Bevy materials and embedded WGSL for stylized rendering experiments.

mod watercolor_shader;

pub use watercolor_shader::{
	WatercolorLightingUniform, WatercolorPaperUniform, WatercolorShadowUniform, WatercolorShader,
	WatercolorShaderPlugin,
};
