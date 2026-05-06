//! Durham terrain material with world-space palette noise ([RFC-170 4.7](https://github.com/ramate-io/maybraid/issues/178)).

use bevy::{
	asset::embedded_asset,
	prelude::*,
	reflect::TypePath,
	render::render_resource::{AsBindGroup, ShaderType},
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

/// One palette swatch: blend **`left.xyz` → `right.xyz`**; **`swatch_meta.x`** = fold-in weight (0–1).
/// `swatch_meta.yzw` unused.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DurhamSwatchUniform {
	pub left: Vec4,
	pub right: Vec4,
	pub swatch_meta: Vec4,
}

/// Per-frequency band: own FBM scale, blend weight vs other bands, and **8 swatches** (independent palette).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DurhamTerrainBandUniform {
	/// `x` = noise seed for this band; `yzw` unused.
	pub config: Vec4,
	/// `x` = frequency, `y` = amplitude, `z` = weight vs other bands, `w` unused.
	pub band_scale: Vec4,
	pub swatches: [DurhamSwatchUniform; 8],
}

/// Full terrain noise config: **regional blend driver** + **4 bands** with swatch tables.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DurhamTerrainNoiseUniform {
	/// `x` = **frequency**, `y` = **amplitude** for the broadest FBM that perturbs inter-band blend weights (`t_warp`); `zw` unused.
	pub regional_blend: Vec4,
	pub bands: [DurhamTerrainBandUniform; 4],
}

fn default_swatches() -> [DurhamSwatchUniform; 8] {
	[
		DurhamSwatchUniform {
			left: Vec4::new(0.36, 0.28, 0.20, 0.0),
			right: Vec4::new(0.42, 0.38, 0.32, 0.0),
			swatch_meta: Vec4::new(1.0, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.42, 0.38, 0.32, 0.0),
			right: Vec4::new(0.45, 0.30, 0.22, 0.0),
			swatch_meta: Vec4::new(1.0, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.45, 0.30, 0.22, 0.0),
			right: Vec4::new(0.48, 0.44, 0.26, 0.0),
			swatch_meta: Vec4::new(1.0, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.48, 0.44, 0.26, 0.0),
			right: Vec4::new(0.20, 0.18, 0.16, 0.0),
			swatch_meta: Vec4::new(1.0, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.20, 0.18, 0.16, 0.0),
			right: Vec4::new(0.36, 0.28, 0.20, 0.0),
			swatch_meta: Vec4::new(0.5, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.39, 0.33, 0.26, 0.0),
			right: Vec4::new(0.435, 0.34, 0.24, 0.0),
			swatch_meta: Vec4::new(0.5, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.435, 0.34, 0.24, 0.0),
			right: Vec4::new(0.34, 0.31, 0.21, 0.0),
			swatch_meta: Vec4::new(0.5, 0.0, 0.0, 0.0),
		},
		DurhamSwatchUniform {
			left: Vec4::new(0.34, 0.31, 0.21, 0.0),
			right: Vec4::new(0.42, 0.38, 0.32, 0.0),
			swatch_meta: Vec4::new(0.5, 0.0, 0.0, 0.0),
		},
	]
}

impl Default for DurhamTerrainNoiseUniform {
	fn default() -> Self {
		let swatches = default_swatches();
		Self {
			regional_blend: Vec4::new(0.00015, 0.5, 0.0, 0.0),
			bands: [
				DurhamTerrainBandUniform {
					config: Vec4::new(42.0, 0.0, 0.0, 0.0),
					band_scale: Vec4::new(0.0001, 0.5, 0.35, 0.0),
					swatches,
				},
				DurhamTerrainBandUniform {
					config: Vec4::new(42.0, 0.0, 0.0, 0.0),
					band_scale: Vec4::new(0.001, 0.5, 0.25, 0.0),
					swatches,
				},
				DurhamTerrainBandUniform {
					config: Vec4::new(42.0, 0.0, 0.0, 0.0),
					band_scale: Vec4::new(0.01, 0.5, 0.25, 0.0),
					swatches,
				},
				DurhamTerrainBandUniform {
					config: Vec4::new(42.0, 0.0, 0.0, 0.0),
					band_scale: Vec4::new(0.05, 0.4, 0.15, 0.0),
					swatches,
				},
			],
		}
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
