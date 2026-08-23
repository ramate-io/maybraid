//! Canopy leaf [`Material`] — object-space leafy breakup, vertex sway, double-sided PBR.

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};

/// Registers embedded **`chico_leaf_material.wgsl`** and [`MaterialPlugin`] for [`ChicoLeafMaterial`].
pub struct ChicoLeafMaterialPlugin;

impl Plugin for ChicoLeafMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "chico_leaf_material.wgsl");
		app.add_plugins(MaterialPlugin::<ChicoLeafMaterial>::default());
		app.add_systems(PostUpdate, disable_leaf_shadow_casters);
	}
}

fn disable_leaf_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<ChicoLeafMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ChicoLeafMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl Default for ChicoLeafMaterial {
	fn default() -> Self {
		Self { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
	}
}

impl Material for ChicoLeafMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "chico_leaf_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "chico_leaf_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		// `fwidth` coverage; falls back to discard when MSAA is off.
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
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
		// ✅ Disable backface culling → renders both sides
		descriptor.primitive.cull_mode = None;

		Ok(())
	}
}
