use bevy::{
	prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

/// Good starting values:
/// water_color = vec4(0.05, 0.25, 0.35, 1.0)
/// mix_color_a = vec4(0.35, 0.30, 0.20, 1.0) (sediment)
/// mix_color_b = vec4(0.12, 0.35, 0.28, 1.0) (algae tint)
/// swirl_params = vec4(swirl_scale=0.015, swirl_speed=0.25, swirl_strength=0.55, foam_strength=0.9)
/// foam_params = vec4(edge_min=0.002, edge_max=0.050, foam_scale=0.05, foam_speed=0.20)
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
	#[uniform(0)]
	pub base_color: Vec4, // HSL or RGB in a vec4
	#[uniform(1)]
	pub mix_color_a: Vec4,
	#[uniform(2)]
	pub mix_color_b: Vec4,
	#[uniform(3)]
	pub swirl_params: Vec4,
	#[uniform(4)]
	pub foam_params: Vec4,
	#[uniform(5)]
	pub time: f32,
}

impl Default for WaterMaterial {
	fn default() -> Self {
		Self {
			base_color: Vec4::new(0.05, 0.25, 0.35, 0.75),
			mix_color_a: Vec4::new(0.35, 0.30, 0.20, 0.25),
			mix_color_b: Vec4::new(0.12, 0.35, 0.28, 0.25),
			swirl_params: Vec4::new(0.015, 0.25, 0.55, 0.9),
			foam_params: Vec4::new(0.002, 0.05, 0.05, 0.2),
			time: 0.0,
		}
	}
}

impl Material for WaterMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/water_material.wgsl".into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::AlphaToCoverage
	}
}
