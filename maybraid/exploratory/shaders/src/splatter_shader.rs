//! Simple albedo-textured [`Material`] for embedded preview textures.

use bevy::{
	asset::{embedded_asset, load_embedded_asset},
	prelude::*,
	reflect::TypePath,
	render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Loaded by [`SplatterShaderPlugin`]; clone the handle when building body/clothing materials.
#[derive(Resource, Clone, Debug)]
pub struct SplatterAlbedo(pub Handle<Image>);

/// Loaded by [`SplatterShaderPlugin`]; clone the handle when building eye materials.
#[derive(Resource, Clone, Debug)]
pub struct EyeballAlbedo(pub Handle<Image>);

/// Registers embedded albedo textures, **`splatter_shader.wgsl`**, and [`MaterialPlugin`] for [`SplatterShader`].
pub struct SplatterShaderPlugin;

impl Plugin for SplatterShaderPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "splatter_shader.wgsl");
		embedded_asset!(app, "splatter.png");
		embedded_asset!(app, "eyeball.png");
		app.add_plugins(MaterialPlugin::<SplatterShader>::default())
			.add_systems(Startup, load_preview_albedos);
	}
}

fn load_preview_albedos(asset_server: Res<AssetServer>, mut commands: Commands) {
	commands.insert_resource(SplatterAlbedo(load_embedded_asset!(&*asset_server, "splatter.png")));
	commands.insert_resource(EyeballAlbedo(load_embedded_asset!(&*asset_server, "eyeball.png")));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SplatterShader {
	/// RGB tint multiplied into the albedo sample; **w** = alpha.
	#[uniform(0)]
	pub base_color: Vec4,
	#[texture(1)]
	#[sampler(2)]
	pub albedo: Handle<Image>,
}

impl SplatterShader {
	/// Builds a material using the given albedo texture handle.
	#[inline]
	pub fn new(albedo: Handle<Image>) -> Self {
		Self { base_color: Vec4::ONE, albedo }
	}

	/// Sets the material tint from a Bevy [`Color`].
	#[inline]
	pub fn with_base_color(mut self, color: Color) -> Self {
		let c = color.to_srgba();
		self.base_color = Vec4::new(c.red, c.green, c.blue, c.alpha);
		self
	}
}

impl Material for SplatterShader {
	fn fragment_shader() -> ShaderRef {
		// Must match `embedded_path!("splatter_shader.wgsl")` (module name, not package name).
		concat!("embedded://exploratory_shaders/splatter_shader.wgsl").into()
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
	fn with_base_color_sets_tint() {
		let tint = Color::srgb(0.5, 0.2, 0.2);
		let m = SplatterShader::new(Handle::default()).with_base_color(tint);
		let c = tint.to_srgba();
		assert!((m.base_color.x - c.red).abs() < 1e-5);
		assert!((m.base_color.y - c.green).abs() < 1e-5);
		assert!((m.base_color.z - c.blue).abs() < 1e-5);
	}

	#[test]
	fn fragment_shader_ref_matches_embedded_asset_path() {
		let embedded = embedded_path!("splatter_shader.wgsl");
		let expected = format!("embedded://{}", embedded.display());
		match <SplatterShader as Material>::fragment_shader() {
			ShaderRef::Path(p) => assert_eq!(p.to_string(), expected),
			ShaderRef::Default => panic!("unexpected Default shader ref"),
			ShaderRef::Handle(_) => panic!("unexpected Handle shader ref"),
		}
	}

	#[test]
	fn splatter_albedo_embedded_path() {
		assert_eq!(embedded_path!("splatter.png"), Path::new("exploratory_shaders/splatter.png"));
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
		embedded_asset!(app, "splatter_shader.wgsl");
		embedded_asset!(app, "splatter.png");
		embedded_asset!(app, "eyeball.png");
		app.update();
	}
}
