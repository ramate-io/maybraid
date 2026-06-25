//! Stick / bark [`Material`] — PBR with screen-space edge darkening (`edge_material` lineage).

use bevy::{
	asset::embedded_asset, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup,
	shader::ShaderRef,
};

/// Registers embedded **`chico_stick_material.wgsl`** and [`MaterialPlugin`] for [`ChicoStickMaterial`].
pub struct ChicoStickMaterialPlugin;

impl Plugin for ChicoStickMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "chico_stick_material.wgsl");
		app.add_plugins(MaterialPlugin::<ChicoStickMaterial>::default());
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ChicoStickMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl Default for ChicoStickMaterial {
	fn default() -> Self {
		Self { base_color: Vec4::new(0.13, 0.085, 0.055, 1.0) }
	}
}

impl Material for ChicoStickMaterial {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "chico_stick_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}
