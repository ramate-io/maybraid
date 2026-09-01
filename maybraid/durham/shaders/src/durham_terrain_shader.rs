//! Durham terrain material with world-space palette noise ([RFC-170 4.7](https://github.com/ramate-io/maybraid/issues/178)).

mod band;
mod noise_uniform;
mod swatch;

pub use band::DurhamTerrainBandUniform;
pub use noise_uniform::{DurhamTerrainNoiseUniform, EVEN_BAND_BLEND_WEIGHT};
pub use swatch::{DurhamSwatchUniform, EVEN_SWATCH_FOLD_WEIGHT};

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
		app.add_plugins(crate::RefractionWaterPlugin);
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DurhamTerrainShader {
	#[uniform(0)]
	pub terrain_noise: DurhamTerrainNoiseUniform,
	/// `x` = normal soften (blend toward up), `y` = edge strength, `z` = edge darkness, `w` = lit mix.
	#[uniform(1)]
	pub style_params: Vec4,
	/// RGB tint multiplied into the palette noise color; **w** = alpha.
	#[uniform(2)]
	pub base_color: Vec4,
}

impl Default for DurhamTerrainShader {
	fn default() -> Self {
		Self {
			terrain_noise: DurhamTerrainNoiseUniform::default(),
			style_params: Vec4::new(0.35, 2.0, 0.05, 0.5),
			base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
		}
	}
}

impl DurhamTerrainShader {
	/// Sets [`DurhamTerrainNoiseUniform::global_seed`] **x** only; per-band seeds unchanged.
	#[inline]
	pub fn with_noise_global_seed(mut self, seed: f32) -> Self {
		self.terrain_noise.set_global_seed(seed);
		self
	}

	/// Sets global and every band FBM seed to **`seed`** ([`DurhamTerrainNoiseUniform::with_seed_uniform_across_bands`]).
	pub fn with_noise_seed_uniform(mut self, seed: f32) -> Self {
		self.terrain_noise = self.terrain_noise.with_seed_uniform_across_bands(seed);
		self
	}

	/// Multiplies the procedural palette by this RGB(A) tint.
	#[inline]
	pub fn with_base_color(mut self, base_color: Vec4) -> Self {
		self.base_color = base_color;
		self
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
		assert!((m.terrain_noise.seed() - 42.0).abs() < 1e-5);
		assert!((m.terrain_noise.bands[0].config.x - 120_079.0).abs() < 1e-3);
		assert!((m.terrain_noise.regional_blend.x - 0.00015).abs() < 1e-8);
		assert!((m.terrain_noise.regional_blend.y - 0.5).abs() < 1e-8);
		assert!((m.style_params.x - 0.35).abs() < 1e-5);
		assert!((m.style_params.w - 0.5).abs() < 1e-5);
		assert!((m.base_color.x - 1.0).abs() < 1e-5);
	}

	#[test]
	fn with_base_color_sets_tint() {
		let tint = Vec4::new(0.5, 0.2, 0.2, 1.0);
		let m = DurhamTerrainShader::default().with_base_color(tint);
		assert_eq!(m.base_color, tint);
	}

	#[test]
	fn noise_uniform_global_seed_helpers() {
		let n = DurhamTerrainNoiseUniform::default()
			.with_global_seed(7.0)
			.with_band_seed(1, 99.0);
		assert!((n.seed() - 7.0).abs() < 1e-5);
		assert!((n.bands[0].config.x - 120_079.0).abs() < 1e-3);
		assert!((n.bands[1].config.x - 99.0).abs() < 1e-5);

		let n2 = DurhamTerrainNoiseUniform::default().with_seed_uniform_across_bands(3.0);
		assert!((n2.seed() - 3.0).abs() < 1e-5);
		for b in &n2.bands {
			assert!((b.config.x - 3.0).abs() < 1e-5);
		}

		let m = DurhamTerrainShader::default().with_noise_global_seed(11.0);
		assert!((m.terrain_noise.seed() - 11.0).abs() < 1e-5);
		let m2 = DurhamTerrainShader::default().with_noise_seed_uniform(13.0);
		assert!((m2.terrain_noise.seed() - 13.0).abs() < 1e-5);
		assert!((m2.terrain_noise.bands[3].config.x - 13.0).abs() < 1e-5);
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
				render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
					force_fallback_adapter: true,
					priority: WgpuSettingsPriority::WebGL2,
					..default()
				})),
				..default()
			},
			CorePipelinePlugin::default(),
			PbrPlugin::default(),
			DurhamTerrainShaderPlugin,
		));
		app.update();
	}
}
