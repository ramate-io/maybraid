//! Eye albedo [`Material`] with adjustable UV center and scale.

use bevy::{
	asset::{embedded_asset, load_embedded_asset},
	prelude::*,
	reflect::TypePath,
	render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Loaded by [`EyeballShaderPlugin`]; clone when building [`EyeballShader`] instances.
#[derive(Resource, Clone, Debug)]
pub struct EyeballAlbedo(pub Handle<Image>);

/// Registers embedded **`eyeball_shader.wgsl`**, **`eyeball.png`**, and [`MaterialPlugin`] for [`EyeballShader`].
pub struct EyeballShaderPlugin;

impl Plugin for EyeballShaderPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "eyeball_shader.wgsl");
		embedded_asset!(app, "eyeball.png");
		app.add_plugins(MaterialPlugin::<EyeballShader>::default())
			.add_systems(Startup, load_eyeball_albedo);
	}
}

fn load_eyeball_albedo(asset_server: Res<AssetServer>, mut commands: Commands) {
	commands.insert_resource(EyeballAlbedo(load_embedded_asset!(&*asset_server, "eyeball.png")));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EyeballShader {
	/// Pivot in texture space; also the point that scale zooms around.
	///
	/// Mapping: `(uv - uv_center) * uv_scale + uv_center`
	#[uniform(0)]
	pub uv_center: Vec2,
	/// Per-axis zoom around [`Self::uv_center`]; values above `1.0` zoom in.
	#[uniform(1)]
	pub uv_scale: Vec2,
	#[texture(2)]
	#[sampler(3)]
	pub albedo: Handle<Image>,
}

impl EyeballShader {
	#[inline]
	pub fn new(albedo: Handle<Image>) -> Self {
		Self { uv_center: Vec2::new(0.5, 0.5), uv_scale: Vec2::ONE, albedo }
	}

	#[inline]
	pub fn with_uv_center(mut self, uv_center: Vec2) -> Self {
		self.uv_center = uv_center;
		self
	}

	#[inline]
	pub fn with_uv_scale(mut self, uv_scale: Vec2) -> Self {
		self.uv_scale = uv_scale;
		self
	}
}

impl Material for EyeballShader {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://exploratory_shaders/eyeball_shader.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Blend
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::embedded_path;
	use bevy::asset::AssetPlugin;
	use bevy::pbr::Material;
	use bevy::prelude::App;
	use bevy::shader::ShaderRef;
	use bevy::MinimalPlugins;
	use std::path::Path;

	#[test]
	fn builders_set_uv_fields() {
		let m = EyeballShader::new(Handle::default())
			.with_uv_center(Vec2::new(0.5, 0.4))
			.with_uv_scale(Vec2::splat(1.2));
		assert_eq!(m.uv_center, Vec2::new(0.5, 0.4));
		assert_eq!(m.uv_scale, Vec2::splat(1.2));
	}

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		let embedded = embedded_path!("eyeball_shader.wgsl");
		let expected = format!("embedded://{}", embedded.display());
		match <EyeballShader as Material>::fragment_shader() {
			ShaderRef::Path(p) => assert_eq!(p.to_string(), expected),
			ShaderRef::Default => panic!("unexpected Default shader ref"),
			ShaderRef::Handle(_) => panic!("unexpected Handle shader ref"),
		}
	}

	#[test]
	fn eyeball_albedo_embedded_path() {
		assert_eq!(embedded_path!("eyeball.png"), Path::new("exploratory_shaders/eyeball.png"));
	}

	#[test]
	fn embedded_assets_register_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		embedded_asset!(app, "eyeball_shader.wgsl");
		embedded_asset!(app, "eyeball.png");
		app.update();
	}
}
