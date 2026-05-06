//! Durham terrain material with world-space palette noise ([RFC-170 4.7](https://github.com/ramate-io/maybraid/issues/178)).

mod band;
mod noise_uniform;
mod palettes;
mod swatch;

pub use band::DurhamTerrainBandUniform;
pub use noise_uniform::{DurhamTerrainNoiseUniform, EVEN_BAND_BLEND_WEIGHT};
pub use palettes::{macro_region_palette, micro_region_palette, EVEN_SWATCH_FOLD_WEIGHT};
pub use swatch::DurhamSwatchUniform;

use bevy::{
	asset::embedded_asset,
	prelude::*,
	reflect::TypePath,
	render::render_resource::AsBindGroup,
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
	pub terrain_noise: DurhamTerrainNoiseUniform,
	/// `x` = reserved, `y` = edge strength, `z` = edge darkness, `w` = lit mix.
	#[uniform(1)]
	pub style_params: Vec4,
}

impl Default for DurhamTerrainShader {
	fn default() -> Self {
		Self {
			terrain_noise: DurhamTerrainNoiseUniform::default(),
			style_params: Vec4::new(0.0, 2.0, 0.05, 0.72),
		}
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
		assert!((m.terrain_noise.bands[0].config.x - 42.0).abs() < 1e-5);
		assert!((m.terrain_noise.regional_blend.x - 0.00015).abs() < 1e-8);
		assert!((m.terrain_noise.regional_blend.y - 0.5).abs() < 1e-8);
		assert!((m.style_params.w - 0.72).abs() < 1e-5);
	}

	#[test]
	fn macro_band_uses_macro_palette_first_swatch_differs_from_micro() {
		let m = DurhamTerrainShader::default();
		assert_ne!(
			m.terrain_noise.bands[0].swatches[0].left,
			m.terrain_noise.bands[1].swatches[0].left
		);
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

	#[test]
	fn embedded_wgsl_registers_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		embedded_asset!(app, "durham_terrain_shader.wgsl");
		app.update();
	}

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
