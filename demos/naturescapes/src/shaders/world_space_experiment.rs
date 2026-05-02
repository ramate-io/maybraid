//! Rust binding logic for world space experiment shader.
//!

use bevy::{
	prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WorldSpaceExperimentMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
	/// x = seed, y = regional scale, z = detail scale, w = value strength.
	#[uniform(1)]
	pub noise_params: Vec4,
	/// x = palette strength, y = edge strength, z = edge darkness, w = lit mix.
	#[uniform(2)]
	pub style_params: Vec4,
}

impl Default for WorldSpaceExperimentMaterial {
	fn default() -> Self {
		Self {
			base_color: Vec4::new(0.89, 0.886, 0.604, 1.0),
			noise_params: Vec4::new(42.0, 0.012, 0.11, 0.24),
			style_params: Vec4::new(0.88, 2.0, 0.05, 0.72),
		}
	}
}

impl WorldSpaceExperimentMaterial {
	pub fn with_base_color(self, base_color: Vec4) -> Self {
		Self { base_color, ..self }
	}
}

impl Material for WorldSpaceExperimentMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/world_space_experiment.wgsl".into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}
