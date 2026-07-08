//! Soft watercolor [`Material`] — half-Lambert lighting, value bands, paper noise, and cool shadows.

mod lighting_uniform;
mod paper_uniform;
mod shadow_uniform;

pub use lighting_uniform::WatercolorLightingUniform;
pub use paper_uniform::WatercolorPaperUniform;
pub use shadow_uniform::WatercolorShadowUniform;

use bevy::{
	asset::embedded_asset,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};

/// Registers embedded **`watercolor_shader.wgsl`** and [`MaterialPlugin`] for [`WatercolorShader`].
pub struct WatercolorShaderPlugin;

impl Plugin for WatercolorShaderPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "watercolor_shader.wgsl");
		app.add_plugins(MaterialPlugin::<WatercolorShader>::default());
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WatercolorShader {
	/// RGB tint multiplied into the painted color; **w** = alpha.
	#[uniform(0)]
	pub base_color: Vec4,
	#[uniform(1)]
	pub lighting: WatercolorLightingUniform,
	#[uniform(2)]
	pub shadow: WatercolorShadowUniform,
	#[uniform(3)]
	pub paper: WatercolorPaperUniform,
}

impl Default for WatercolorShader {
	fn default() -> Self {
		Self {
			base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
			lighting: WatercolorLightingUniform::default(),
			shadow: WatercolorShadowUniform::default(),
			paper: WatercolorPaperUniform::default(),
		}
	}
}

impl WatercolorShader {
	/// Sets the material tint from a Bevy [`Color`].
	#[inline]
	pub fn with_base_color(mut self, color: Color) -> Self {
		let c = color.to_srgba();
		self.base_color = Vec4::new(c.red, c.green, c.blue, c.alpha);
		self
	}

	/// Sets the material tint from RGBA components.
	#[inline]
	pub fn with_base_color_vec4(mut self, base_color: Vec4) -> Self {
		self.base_color = base_color;
		self
	}

	#[inline]
	pub fn with_lighting(mut self, lighting: WatercolorLightingUniform) -> Self {
		self.lighting = lighting;
		self
	}

	#[inline]
	pub fn with_shadow(mut self, shadow: WatercolorShadowUniform) -> Self {
		self.shadow = shadow;
		self
	}

	#[inline]
	pub fn with_shadow_color(mut self, color: Color) -> Self {
		self.shadow = WatercolorShadowUniform::from_color(color);
		self
	}

	#[inline]
	pub fn with_paper(mut self, paper: WatercolorPaperUniform) -> Self {
		self.paper = paper;
		self
	}
}

impl Material for WatercolorShader {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "watercolor_shader.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
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

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::AssetPlugin;
	use bevy::pbr::Material;
	use bevy::prelude::App;
	use bevy::shader::ShaderRef;
	use bevy::MinimalPlugins;

	#[test]
	fn default_material_uniforms() {
		let m = WatercolorShader::default();
		assert!((m.lighting.band_count - 4.0).abs() < 1e-5);
		assert!((m.lighting.band_mix - 0.35).abs() < 1e-5);
		assert!((m.lighting.light_smooth_min - 0.3).abs() < 1e-5);
		assert!((m.lighting.light_smooth_max - 0.78).abs() < 1e-5);
		assert!((m.lighting.diffuse_scale - 0.55).abs() < 1e-5);
		assert!((m.lighting.diffuse_bias - 0.4).abs() < 1e-5);
		assert!((m.shadow.tint_r - 0.42).abs() < 1e-5);
		assert!((m.paper.noise_strength - 0.15).abs() < 1e-5);
		assert!((m.base_color.w - 1.0).abs() < 1e-5);
	}

	#[test]
	fn with_base_color_sets_tint() {
		let tint = Color::srgb(0.5, 0.2, 0.2);
		let m = WatercolorShader::default().with_base_color(tint);
		let c = tint.to_srgba();
		assert!((m.base_color.x - c.red).abs() < 1e-5);
		assert!((m.base_color.y - c.green).abs() < 1e-5);
		assert!((m.base_color.z - c.blue).abs() < 1e-5);
	}

	#[test]
	fn lighting_uniform_builders() {
		let lighting = WatercolorLightingUniform::default()
			.with_band_count(6.0)
			.with_diffuse_wrap(0.5, 0.45);
		assert!((lighting.band_count - 6.0).abs() < 1e-5);
		assert!((lighting.diffuse_scale - 0.5).abs() < 1e-5);
		assert!((lighting.diffuse_bias - 0.45).abs() < 1e-5);
	}

	#[test]
	fn material_alpha_mode_is_opaque() {
		let m = WatercolorShader::default();
		assert_eq!(m.alpha_mode(), AlphaMode::Opaque);
	}

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		let expected =
			concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "watercolor_shader.wgsl");
		match <WatercolorShader as Material>::fragment_shader() {
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
		embedded_asset!(app, "watercolor_shader.wgsl");
		app.update();
	}
}
