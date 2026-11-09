//! Existing vegetation shading with a resident instanced vertex input.

use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::shader::{ShaderDefVal, ShaderRef};
use vegetation_shaders::{LeafMaterial, StickMaterial};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PackedLeafMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PackedStickMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

/// Shadow-layer-only counterpart of the ordinary shared canopy icosphere.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PackedShadowMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

fn specialize_instances(descriptor: &mut RenderPipelineDescriptor) {
	let prepass =
		descriptor.vertex.shader_defs.iter().any(
			|value| matches!(value, ShaderDefVal::Bool(name, true) if name == "PREPASS_PIPELINE"),
		);
	descriptor.vertex.shader_defs.push("PACKED_GROVE".into());
	descriptor.vertex.entry_point = Some(if prepass { "vertex" } else { "packed_vertex" }.into());
	descriptor.vertex.buffers.push(VertexBufferLayout {
		array_stride: 176,
		step_mode: VertexStepMode::Instance,
		attributes: (0..11)
			.map(|index| VertexAttribute {
				format: VertexFormat::Float32x4,
				offset: index * 16,
				shader_location: 5 + index as u32,
			})
			.collect(),
	});
}

impl Material for PackedLeafMaterial {
	fn vertex_shader() -> ShaderRef {
		LeafMaterial::vertex_shader()
	}
	fn fragment_shader() -> ShaderRef {
		LeafMaterial::fragment_shader()
	}
	fn enable_prepass() -> bool {
		false
	}
	fn enable_shadows() -> bool {
		false
	}
	fn reads_view_transmission_texture(&self) -> bool {
		false
	}
	fn specialize(
		pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		layout: &MeshVertexBufferLayoutRef,
		key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		let _ = (pipeline, layout, key);
		descriptor.primitive.cull_mode = None;
		specialize_instances(descriptor);
		Ok(())
	}
}

impl Material for PackedStickMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/packed/packed_vertex.wesl").into()
	}
	fn fragment_shader() -> ShaderRef {
		StickMaterial::fragment_shader()
	}
	fn prepass_vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/packed/packed_prepass.wesl").into()
	}
	// Ordinary VegetationMaterialLib marks bark NotShadowCaster as well.
	fn enable_shadows() -> bool {
		false
	}
	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		specialize_instances(descriptor);
		Ok(())
	}
}

impl Material for PackedShadowMaterial {
	fn vertex_shader() -> ShaderRef {
		PackedStickMaterial::vertex_shader()
	}
	fn fragment_shader() -> ShaderRef {
		StickMaterial::fragment_shader()
	}
	fn prepass_vertex_shader() -> ShaderRef {
		PackedStickMaterial::prepass_vertex_shader()
	}
	fn enable_prepass() -> bool {
		false
	}
	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		// Match the ordinary procedural StandardMaterial caster's back culling.
		specialize_instances(descriptor);
		Ok(())
	}
}
