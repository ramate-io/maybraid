//! Per-instance additive/blend material for columns, contact disks, and wave rings.
//!
//! Named [`FieldMaterial`] so it does not collide with `firearms::EnergyMaterial`.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::palette::FieldPalette;

/// Visual role packed into [`FieldMaterial::params`].w.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
	ColumnCore,
	ColumnSheath,
	GroundContact,
	ShockRing,
	ShockTrail,
}

impl FieldKind {
	pub fn as_f32(self) -> f32 {
		match self {
			Self::ColumnCore => 0.0,
			Self::ColumnSheath => 1.0,
			Self::GroundContact => 2.0,
			Self::ShockRing => 3.0,
			Self::ShockTrail => 4.0,
		}
	}
}

/// Age, envelope, kind, and palette packed for one field mesh.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FieldMaterial {
	/// `x` age, `y` duration, `z` seed, `w` [`FieldKind`] as f32.
	#[uniform(0)]
	pub params: Vec4,
	/// `x` alpha, `y` reveal, `z` contact, `w` intensity.
	#[uniform(1)]
	pub envelope: Vec4,
	#[uniform(2)]
	pub tint: Vec4,
	/// `x` gain, `y` scroll, `z` softness, `w` bands.
	#[uniform(3)]
	pub extras: Vec4,
	#[uniform(4)]
	pub color_hot: Vec4,
	#[uniform(5)]
	pub color_mid: Vec4,
	#[uniform(6)]
	pub color_cool: Vec4,
}

impl FieldMaterial {
	pub fn new(
		kind: FieldKind,
		duration: f32,
		seed: f32,
		palette: &FieldPalette,
		tint: Color,
		intensity: f32,
	) -> Self {
		let tint = LinearRgba::from(tint);
		let (gain, scroll, soft, bands) = match kind {
			FieldKind::ColumnCore => (1.22 * intensity, 0.55, 0.18, 5.0),
			FieldKind::ColumnSheath => (0.72 * intensity, 0.32, 0.38, 4.0),
			FieldKind::GroundContact => (1.85 * intensity, 0.80, 0.26, 3.0),
			FieldKind::ShockRing => (0.95 * intensity, 1.25, 0.28, 5.0),
			FieldKind::ShockTrail => (0.62 * intensity, 1.45, 0.32, 4.0),
		};
		Self {
			params: Vec4::new(0.0, duration.max(1e-3), seed, kind.as_f32()),
			envelope: Vec4::new(0.0, 0.0, 0.0, intensity),
			tint: Vec4::new(tint.red, tint.green, tint.blue, 1.0),
			extras: Vec4::new(gain, scroll, soft, bands),
			color_hot: crate::palette::ExplosionPalette::vec4(palette.hot),
			color_mid: crate::palette::ExplosionPalette::vec4(palette.mid),
			color_cool: crate::palette::ExplosionPalette::vec4(palette.cool),
		}
	}

	pub fn set_age(&mut self, age: f32) {
		self.params.x = age.max(0.0);
	}

	pub fn set_envelope(&mut self, alpha: f32, reveal: f32, contact: f32) {
		self.envelope.x = alpha.clamp(0.0, 1.0);
		self.envelope.y = reveal.clamp(0.0, 1.0);
		self.envelope.z = contact.clamp(0.0, 1.0);
	}

	pub fn scale_gain(&mut self, factor: f32) {
		self.extras.x *= factor.max(0.0);
	}

	pub fn set_softness(&mut self, softness: f32) {
		self.extras.z = softness.clamp(0.05, 0.95);
	}

	pub fn alpha_mode_for_kind(&self) -> AlphaMode {
		if self.params.w < 0.5 {
			AlphaMode::Add
		} else if self.params.w < 1.5 {
			AlphaMode::Blend
		} else if self.params.w < 3.5 {
			AlphaMode::Blend
		} else {
			AlphaMode::Add
		}
	}
}

impl Default for FieldMaterial {
	fn default() -> Self {
		Self::new(FieldKind::ColumnCore, 11.0, 0.0, &FieldPalette::heaven(), Color::WHITE, 1.0)
	}
}

impl Material for FieldMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "field_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "field_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		self.alpha_mode_for_kind()
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
		// Two-sided so a 7 m column still reads when the camera is inside it.
		descriptor.primitive.cull_mode = None;
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

/// Embedded field shader plus [`MaterialPlugin`].
pub struct FieldMaterialPlugin;

impl Plugin for FieldMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "field_material.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<FieldMaterial>>() {
			app.add_plugins(MaterialPlugin::<FieldMaterial>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn core_is_additive_sheath_blends() {
		let palette = FieldPalette::heaven();
		let core =
			FieldMaterial::new(FieldKind::ColumnCore, 11.0, 0.0, &palette, Color::WHITE, 1.0);
		let sheath =
			FieldMaterial::new(FieldKind::ColumnSheath, 11.0, 1.0, &palette, Color::WHITE, 1.0);
		assert_eq!(core.alpha_mode(), AlphaMode::Add);
		assert_eq!(sheath.alpha_mode(), AlphaMode::Blend);
		assert!(core.extras.x > sheath.extras.x);
	}
}
