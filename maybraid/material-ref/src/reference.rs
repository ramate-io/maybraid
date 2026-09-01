//! [`ReferenceMaterial`]: Bevy materials that accept palette + noise from a [`MaterialRef`].

use bevy::prelude::{Color, Material, StandardMaterial};
use procedural_common::NoiseParams;

use crate::material_ref::{MaterialRasters, MaterialRef, MaterialScalars};

/// Material asset that can be parameterized by a [`MaterialRef`].
pub trait ReferenceMaterial: Material + Clone + Default {
	fn with_palette(self, palette: &[Color]) -> Self;
	fn with_noise_params(self, noise: &NoiseParams) -> Self;
	fn with_rasters(self, _rasters: &MaterialRasters) -> Self {
		self
	}
	fn with_scalars(self, _scalars: &MaterialScalars) -> Self {
		self
	}

	fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self::default()
			.with_palette(&material_ref.palette)
			.with_noise_params(&material_ref.noise)
			.with_rasters(&material_ref.rasters)
			.with_scalars(&material_ref.scalars)
	}
}

impl ReferenceMaterial for StandardMaterial {
	fn with_palette(mut self, palette: &[Color]) -> Self {
		if let Some(color) = palette.first() {
			self.base_color = *color;
		}
		self
	}

	fn with_noise_params(self, _noise: &NoiseParams) -> Self {
		// Standard PBR has no noise uniforms; abiding custom materials override.
		self
	}

	fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let mut mat = Self::default()
			.with_palette(&material_ref.palette)
			.with_noise_params(&material_ref.noise)
			.with_rasters(&material_ref.rasters)
			.with_scalars(&material_ref.scalars);
		// Library default: solid green when no palette was authored.
		if material_ref.palette.is_empty() {
			mat.base_color = Color::srgb(0.22, 0.62, 0.28);
			mat.double_sided = true;
		}
		mat
	}
}
