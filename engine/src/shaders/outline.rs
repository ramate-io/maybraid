use bevy::{
	prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EdgeMaterial {
	#[uniform(0)]
	pub base_color: Vec4, // HSL or RGB in a vec4
}

impl Default for EdgeMaterial {
	fn default() -> Self {
		Self { base_color: Vec4::new(0.89, 0.886, 0.604, 1.0) }
	}
}

impl Material for EdgeMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/edge_material.wgsl".into()
	}
}
