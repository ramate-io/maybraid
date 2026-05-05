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
