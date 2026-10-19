//! Embedded dust playback material.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
	AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::dust::playback::DustPlayback;

pub const DUST_SHADER_ASSET: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "dust/shader.wgsl");

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DustPlaybackMaterial {
	#[uniform(0)]
	pub playback: DustPlayback,
	#[texture(1)]
	#[sampler(2)]
	pub atlas: Handle<Image>,
	#[texture(3, dimension = "3d")]
	#[sampler(4)]
	pub density: Handle<Image>,
	#[texture(5, dimension = "2d_array")]
	#[sampler(6)]
	pub multiview: Handle<Image>,
}

impl DustPlaybackMaterial {
	pub fn new(
		playback: DustPlayback,
		atlas: Handle<Image>,
		density: Handle<Image>,
		multiview: Handle<Image>,
	) -> Self {
		Self { playback, atlas, density, multiview }
	}
}

impl Material for DustPlaybackMaterial {
	fn vertex_shader() -> ShaderRef {
		DUST_SHADER_ASSET.into()
	}

	fn fragment_shader() -> ShaderRef {
		DUST_SHADER_ASSET.into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Blend
	}

	fn enable_prepass() -> bool {
		false
	}

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
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
			depth.depth_compare = Some(CompareFunction::Always);
		}
		Ok(())
	}
}

/// Embedded dust shader plus [`MaterialPlugin`].
pub struct DustPlaybackPlugin;

impl Plugin for DustPlaybackPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "shader.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<DustPlaybackMaterial>>() {
			app.add_plugins(MaterialPlugin::<DustPlaybackMaterial>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		match <DustPlaybackMaterial as Material>::fragment_shader() {
			ShaderRef::Path(path) => assert_eq!(path.to_string(), DUST_SHADER_ASSET),
			ShaderRef::Default => panic!("unexpected Default shader ref"),
			ShaderRef::Handle(_) => panic!("unexpected Handle shader ref"),
		}
	}

	#[test]
	fn embedded_wgsl_registers_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		embedded_asset!(app, "shader.wgsl");
		app.update();
	}
}
