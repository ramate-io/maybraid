use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::render::render_resource::*;
use bevy::{
	mesh::MeshVertexBufferLayoutRef, prelude::*, reflect::TypePath,
	render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LeafMaterial {
	#[uniform(0)]
	pub base_color: Vec4, // HSL or RGB in a vec4
	#[uniform(1)]
	pub drop_color: Vec4,
}

impl Default for LeafMaterial {
	fn default() -> Self {
		Self {
			base_color: Vec4::new(0.2, 0.8, 0.3, 1.0),
			// dark brown color
			drop_color: Vec4::new(0.1, 0.15, 0.1, 0.2),
		}
	}
}

impl LeafMaterial {
	pub fn with_base_color(self, base_color: Vec4) -> Self {
		Self { base_color, ..self }
	}

	pub fn with_drop_color(self, drop_color: Vec4) -> Self {
		Self { drop_color, ..self }
	}
}

impl Material for LeafMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/leaf_material.wgsl".into()
	}

	// Enable alpha blending for transparency
	// This allows the leaf shape alpha to create see-through areas
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		// ✅ Disable backface culling → renders both sides
		descriptor.primitive.cull_mode = None;

		Ok(())
	}
}
