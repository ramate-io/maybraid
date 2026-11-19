//! Per-instance directional soundwave material. Live uniforms follow the
//! gameplay wave: origin, heading, position, radius, and thickness.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Additive compression-edge ring. `wave` / `front` are world meters.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SoundwaveMaterial {
	/// `xyz` wave origin, `w` ring radius.
	#[uniform(0)]
	pub wave: Vec4,
	/// `xyz` travel heading, `w` thickness along travel.
	#[uniform(1)]
	pub front: Vec4,
	#[uniform(2)]
	pub tint: Vec4,
	/// `x` age, `y` intensity, `z` fade, `w` seed.
	#[uniform(3)]
	pub extras: Vec4,
}

impl SoundwaveMaterial {
	pub fn new(
		origin: Vec3,
		direction: Vec3,
		radius: f32,
		thickness: f32,
		tint: Color,
		intensity: f32,
	) -> Self {
		let tint = LinearRgba::from(tint);
		Self {
			wave: Vec4::new(origin.x, origin.y, origin.z, radius.max(0.05)),
			front: Vec4::new(direction.x, direction.y, direction.z, thickness.max(0.05)),
			tint: Vec4::new(tint.red, tint.green, tint.blue, 1.0),
			extras: Vec4::new(0.0, intensity.max(0.25), 1.0, 0.17),
		}
	}

	pub fn follow(&mut self, origin: Vec3, direction: Vec3, radius: f32, thickness: f32) {
		self.wave = Vec4::new(origin.x, origin.y, origin.z, radius.max(0.05));
		self.front = Vec4::new(direction.x, direction.y, direction.z, thickness.max(0.05));
	}

	pub fn set_age(&mut self, age: f32) {
		self.extras.x = age.max(0.0);
	}

	pub fn set_fade(&mut self, fade: f32) {
		self.extras.z = fade.clamp(0.0, 1.0);
	}
}

impl Default for SoundwaveMaterial {
	fn default() -> Self {
		Self::new(Vec3::ZERO, Vec3::NEG_Z, 4.0, 0.7, Color::srgb(0.72, 0.88, 1.0), 1.15)
	}
}

impl Material for SoundwaveMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/soundwave/soundwave_material.wesl")
			.into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/soundwave/soundwave_material.wesl")
			.into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Add
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

/// Embedded soundwave shader plus [`MaterialPlugin`].
pub struct SoundwaveMaterialPlugin;

impl Plugin for SoundwaveMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "soundwave_material.wesl");
		if !app.is_plugin_added::<MaterialPlugin<SoundwaveMaterial>>() {
			app.add_plugins(MaterialPlugin::<SoundwaveMaterial>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn follow_writes_live_wave_uniforms() {
		let mut material = SoundwaveMaterial::default();
		material.follow(Vec3::new(1.0, 2.0, 3.0), Vec3::X, 6.0, 0.8);
		assert!(
			(Vec3::new(material.wave.x, material.wave.y, material.wave.z)
				- Vec3::new(1.0, 2.0, 3.0))
			.length() < 1e-5
		);
		assert!((material.wave.w - 6.0).abs() < 1e-5);
		assert!(
			(Vec3::new(material.front.x, material.front.y, material.front.z) - Vec3::X).length()
				< 1e-5
		);
		assert!((material.front.w - 0.8).abs() < 1e-5);
		assert_eq!(material.alpha_mode(), AlphaMode::Add);
	}
}
