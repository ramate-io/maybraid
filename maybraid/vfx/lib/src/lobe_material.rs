//! Shared lobe material plus packed instance storage.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::render::storage::{ShaderBuffer, StoragePlugin};
use bevy::shader::ShaderRef;

use crate::lobe_instances::setup_lobe_instance_pack;

/// Shared pipeline shell. Per-lobe state lives in the packed [`ShaderBuffer`].
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LobeMaterial {
	#[storage(0, read_only)]
	pub instances: Handle<ShaderBuffer>,
	/// `x` is 1.0 for additive flash lobes, 0.0 for blend fire/smoke.
	#[uniform(1)]
	pub pipeline: Vec4,
}

impl LobeMaterial {
	pub fn blend(instances: Handle<ShaderBuffer>) -> Self {
		Self { instances, pipeline: Vec4::new(0.0, 0.0, 0.0, 0.0) }
	}

	pub fn additive(instances: Handle<ShaderBuffer>) -> Self {
		Self { instances, pipeline: Vec4::new(1.0, 0.0, 0.0, 0.0) }
	}
}

impl Default for LobeMaterial {
	fn default() -> Self {
		Self::blend(Handle::default())
	}
}

impl Material for LobeMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "lobe_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "lobe_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		if self.pipeline.x > 0.5 {
			AlphaMode::Add
		} else {
			AlphaMode::Blend
		}
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
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
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

/// Embedded lobe shader, storage buffers, and instance sync.
pub struct LobeMaterialPlugin;

impl Plugin for LobeMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "lobe_material.wgsl");
		if !app.is_plugin_added::<StoragePlugin>() {
			app.add_plugins(StoragePlugin);
		}
		if !app.is_plugin_added::<MaterialPlugin<LobeMaterial>>() {
			app.add_plugins(MaterialPlugin::<LobeMaterial>::default());
		}
		app.add_systems(Startup, setup_lobe_instance_pack);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::LobeKind;
	use crate::lobe_instances::LobeInstanceGpu;
	use crate::palette::ExplosionPalette;

	#[test]
	fn pipeline_flag_selects_alpha_mode() {
		let blend = LobeMaterial::blend(Handle::default());
		let add = LobeMaterial::additive(Handle::default());
		assert_eq!(blend.alpha_mode(), AlphaMode::Blend);
		assert_eq!(add.alpha_mode(), AlphaMode::Add);
	}

	#[test]
	fn instance_gpu_matches_kind_defaults() {
		let palette = ExplosionPalette::maybraid();
		let flash = LobeInstanceGpu::new(LobeKind::Flash, 0.1, 2.0, &palette, Color::WHITE, 1.0);
		assert!((flash.params.w - 2.0).abs() < 1e-4);
	}
}
