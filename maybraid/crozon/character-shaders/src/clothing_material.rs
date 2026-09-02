//! Clothing [`Material`] — palette tint, look kind, and a tiny vertex sway.

use bevy::{
	asset::embedded_asset,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};

pub const KIND_CLOTH: u32 = 0;
pub const KIND_SPACE_SUIT: u32 = 1;
pub const KIND_TATTERED: u32 = 2;
pub const KIND_HAWAIIAN: u32 = 3;
pub const KIND_WIZARDS_VEINS: u32 = 4;
pub const KIND_GLITTER: u32 = 5;
pub const KIND_SCALES: u32 = 6;

/// Registers embedded **`clothing_material.wgsl`** and [`MaterialPlugin`].
pub struct ClothingShaderMaterialPlugin;

impl Plugin for ClothingShaderMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "clothing_material.wgsl");
		app.add_plugins(MaterialPlugin::<ClothingShaderMaterial>::default());
	}
}

/// Look index packed into [`ClothingMaterialUniform::kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClothingShaderKind {
	Cloth,
	SpaceSuit,
	Tattered,
	Hawaiian,
	WizardsVeins,
	Glitter,
	Scales,
}

impl ClothingShaderKind {
	pub const fn as_u32(self) -> u32 {
		match self {
			Self::Cloth => KIND_CLOTH,
			Self::SpaceSuit => KIND_SPACE_SUIT,
			Self::Tattered => KIND_TATTERED,
			Self::Hawaiian => KIND_HAWAIIAN,
			Self::WizardsVeins => KIND_WIZARDS_VEINS,
			Self::Glitter => KIND_GLITTER,
			Self::Scales => KIND_SCALES,
		}
	}
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ClothingMaterialUniform {
	pub base_color: Vec4,
	pub kind: u32,
	pub _pad: UVec3,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ClothingShaderMaterial {
	#[uniform(0)]
	pub params: ClothingMaterialUniform,
}

impl ClothingShaderMaterial {
	pub fn new(kind: ClothingShaderKind, base_color: Vec4) -> Self {
		Self {
			params: ClothingMaterialUniform { base_color, kind: kind.as_u32(), _pad: UVec3::ZERO },
		}
	}
}

impl Default for ClothingShaderMaterial {
	fn default() -> Self {
		Self::new(ClothingShaderKind::Cloth, Vec4::new(0.46, 0.60, 0.72, 1.0))
	}
}

impl Material for ClothingShaderMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "clothing_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "clothing_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		// Tattered chews holes with fragment `discard` (Chico leaf cheese).
		// Opaque ignores alpha, so Mask would never punch through.
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
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}
