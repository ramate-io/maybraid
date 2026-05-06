//! Durham terrain material with world-space palette noise ([RFC-170 4.7](https://github.com/ramate-io/maybraid/issues/178)).

use bevy::{
	asset::embedded_asset, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Registers the embedded WGSL and [`MaterialPlugin`] for [`DurhamTerrainShader`].
pub struct DurhamTerrainShaderPlugin;

impl Plugin for DurhamTerrainShaderPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "durham_terrain_shader.wgsl");
		app.add_plugins(MaterialPlugin::<DurhamTerrainShader>::default());
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DurhamTerrainShader {
	#[uniform(0)]
	pub base_color: Vec4,
	/// x = seed, y = regional scale, z = detail scale, w = value strength.
	#[uniform(1)]
	pub noise_params: Vec4,
	/// x = palette strength, y = edge strength, z = edge darkness, w = lit mix.
	#[uniform(2)]
	pub style_params: Vec4,
}

impl Default for DurhamTerrainShader {
	fn default() -> Self {
		Self {
			base_color: Vec4::new(0.89, 0.886, 0.604, 1.0),
			noise_params: Vec4::new(42.0, 0.012, 0.11, 0.24),
			style_params: Vec4::new(0.88, 2.0, 0.05, 0.72),
		}
	}
}

impl DurhamTerrainShader {
	pub fn with_base_color(self, base_color: Vec4) -> Self {
		Self { base_color, ..self }
	}
}

impl Material for DurhamTerrainShader {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "durham_terrain_shader.wgsl",).into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::AssetPlugin;
	use bevy::pbr::Material;
	use bevy::prelude::{default, App};
	use bevy::shader::ShaderRef;
	use bevy::MinimalPlugins;

	#[test]
	fn default_material_uniforms() {
		let m = DurhamTerrainShader::default();
		assert!((m.base_color.w - 1.0).abs() < f32::EPSILON);
		assert!((m.base_color.x - 0.89).abs() < 1e-5);
		assert!((m.noise_params.x - 42.0).abs() < 1e-5);
		assert!((m.style_params.x - 0.88).abs() < 1e-5);
	}

	#[test]
	fn with_base_color_overrides_base_only() {
		let base = Vec4::new(0.1, 0.2, 0.3, 0.4);
		let m = DurhamTerrainShader::default().with_base_color(base);
		assert_eq!(m.base_color, base);
		let d = DurhamTerrainShader::default();
		assert_eq!(m.noise_params, d.noise_params);
		assert_eq!(m.style_params, d.style_params);
	}

	#[test]
	fn material_alpha_mode_is_opaque() {
		let m = DurhamTerrainShader::default();
		assert_eq!(m.alpha_mode(), AlphaMode::Opaque);
	}

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		let expected =
			concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "durham_terrain_shader.wgsl",);
		match <DurhamTerrainShader as Material>::fragment_shader() {
			ShaderRef::Path(p) => assert_eq!(p.to_string(), expected),
			ShaderRef::Default => panic!("unexpected Default shader ref"),
			ShaderRef::Handle(_) => panic!("unexpected Handle shader ref"),
		}
	}

	/// Covers the `embedded_asset!` step from [`DurhamTerrainShaderPlugin`] without requiring a GPU.
	#[test]
	fn embedded_wgsl_registers_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		embedded_asset!(app, "durham_terrain_shader.wgsl");
		app.update();
	}

	/// Full [`DurhamTerrainShaderPlugin`] (including [`MaterialPlugin`]) needs a wgpu adapter.
	/// Skipped in headless CI; run locally with:
	/// `cargo test -p durham-terrain-shaders durham_terrain_shader_plugin_smoke_gpu -- --ignored --nocapture`
	#[test]
	#[ignore = "requires GPU / wgpu adapter"]
	fn durham_terrain_shader_plugin_smoke_gpu() {
		use bevy::core_pipeline::CorePipelinePlugin;
		use bevy::image::ImagePlugin;
		use bevy::mesh::MeshPlugin;
		use bevy::pbr::PbrPlugin;
		use bevy::render::settings::{RenderCreation, WgpuSettings, WgpuSettingsPriority};
		use bevy::render::RenderPlugin;
		use bevy::window::WindowPlugin;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			WindowPlugin::default(),
			AssetPlugin::default(),
			ImagePlugin::default(),
			MeshPlugin::default(),
			RenderPlugin {
				render_creation: RenderCreation::Automatic(WgpuSettings {
					force_fallback_adapter: true,
					priority: WgpuSettingsPriority::Compatibility,
					..default()
				}),
				..default()
			},
			CorePipelinePlugin::default(),
			PbrPlugin::default(),
			DurhamTerrainShaderPlugin,
		));
		app.update();
	}
}
