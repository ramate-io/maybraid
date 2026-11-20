//! Herd-only sand material. Forward shading is the approved implementation;
//! depth uses the identical deformation/map and the same reconstructed mesh.
use crate::parts::cartoon_flow::SurfaceEnvironment;
use bevy::{
	asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HerdMaterial {
	#[uniform(0)]
	pub cutoff: Vec4,
	#[uniform(1)]
	pub seed: UVec4,
	#[uniform(2)]
	pub environment: SurfaceEnvironment,
}
impl HerdMaterial {
	pub fn wave(cutoff: Vec4) -> Self {
		Self { cutoff, seed: UVec4::new(0, 0, 2, 1), environment: default() }
	}
}
impl Material for HerdMaterial {
	fn vertex_shader() -> ShaderRef {
		<crate::parts::cartoon_landing::LandingMaterial as Material>::vertex_shader()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	fn prepass_vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/herd/depth.wesl").into()
	}
	fn enable_shadows() -> bool {
		false
	}
}
pub struct HerdMaterialPlugin;
impl Plugin for HerdMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "depth.wesl");
		app.add_plugins(MaterialPlugin::<HerdMaterial>::default());
	}
}
#[cfg(test)]
mod tests {
	#[test]
	fn depth_and_forward_use_identical_authored_deformation() -> Result<(), String> {
		let forward = include_str!("../cartoon_landing/surface.wesl");
		let depth = include_str!("depth.wesl");
		fn function(s: &str) -> Result<&str, String> {
			let start = s.find("fn deform(").ok_or("deform start")?;
			let end = s[start..].find("\n}").ok_or("deform end")?;
			Ok(&s[start..start + end + 2])
		}
		assert_eq!(function(forward)?, function(depth)?);
		Ok(())
	}
}
