use bevy::{
	prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct RefractionWater {
	/// Overall water tint + "how much water color overrides refracted scene"
	/// (alpha is used as opacity in the shader)
	#[uniform(2)]
	pub water_color: LinearRgba,

	/// Shallow water tint (beach / clear water)
	#[uniform(3)]
	pub shallow_color: LinearRgba,

	/// Deep water tint (ocean / lake depths)
	#[uniform(4)]
	pub deep_color: LinearRgba,

	/// UV distortion strength (refraction amount)
	/// keep small for performance
	#[uniform(5)]
	pub distortion_strength: f32,

	/// Depth scale in view-space units (bigger = stays shallow longer)
	/// Higher is smoother
	#[uniform(6)]
	pub depth_scale: f32,

	/// Fade out distortion near the camera to avoid shimmer/jitter
	/// (0 disables fading, 1 is typical)
	#[uniform(7)]
	pub close_fade_strength: f32,
}

impl Default for RefractionWater {
	fn default() -> Self {
		Self {
			water_color: LinearRgba::new(0.05, 0.35, 0.45, 0.99),
			shallow_color: LinearRgba::new(0.15, 0.55, 0.60, 1.0),
			deep_color: LinearRgba::new(0.2, 0.4, 0.5, 1.0),
			distortion_strength: 0.006, // keep small for performance
			depth_scale: 30.0,          // higher is smoother
			close_fade_strength: 1.0,
		}
	}
}

impl Material for RefractionWater {
	fn fragment_shader() -> ShaderRef {
		"shaders/refraction_water.wgsl".into()
	}

	fn reads_view_transmission_texture(&self) -> bool {
		true
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}
}
