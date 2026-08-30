use bevy::prelude::Color;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::{CHICO_BUMP_OUT_MATERIAL, DENSITY_PARAMETER, HEIGHT_PARAMETER, STYLE_PARAMETER};

pub const BUMP_OUT_NEIGHBORHOOD_WIDTH: usize = 3;
pub const BUMP_OUT_NEIGHBORHOOD_SAMPLES: usize =
	BUMP_OUT_NEIGHBORHOOD_WIDTH * BUMP_OUT_NEIGHBORHOOD_WIDTH;

/// Row-major 3×3 density and average-height samples centered on the presented chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutNeighborhood {
	pub densities: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	pub heights: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
}

impl BumpOutNeighborhood {
	pub const fn new(
		densities: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		heights: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	) -> Self {
		Self { densities, heights }
	}

	pub fn uniform(density: f32, height: f32) -> Self {
		Self {
			densities: [density; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
			heights: [height; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		}
	}

	/// Build the deferred material recipe consumed by [`crate::BumpOutMaterialLib`].
	pub fn material_ref(
		self,
		palette: impl IntoIterator<Item = Color>,
		noise: NoiseParams,
	) -> MaterialRef {
		MaterialRef::named(CHICO_BUMP_OUT_MATERIAL)
			.with_palette(palette)
			.with_noise(noise)
			.with_parameter(DENSITY_PARAMETER, self.densities)
			.with_parameter(HEIGHT_PARAMETER, self.heights)
			.with_parameter(STYLE_PARAMETER, BumpOutStyle::default().as_values())
	}

	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self {
			densities: samples_from_ref(material_ref, DENSITY_PARAMETER, 1.0),
			heights: samples_from_ref(material_ref, HEIGHT_PARAMETER, 0.0),
		}
	}

	pub fn min_height(self) -> f32 {
		self.heights.into_iter().fold(f32::INFINITY, f32::min)
	}

	pub fn max_height(self) -> f32 {
		self.heights.into_iter().fold(f32::NEG_INFINITY, f32::max)
	}

	pub fn set_density(&mut self, index: usize, density: f32) {
		if let Some(value) = self.densities.get_mut(index) {
			*value = density.clamp(0.0, 1.0);
		}
	}

	pub fn set_height(&mut self, index: usize, height: f32) {
		if let Some(value) = self.heights.get_mut(index) {
			*value = height;
		}
	}
}

impl Default for BumpOutNeighborhood {
	fn default() -> Self {
		Self::uniform(1.0, 0.0)
	}
}

/// Material-level controls not already represented by [`NoiseParams`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutStyle {
	/// Width of the fragment-coverage transition around the density threshold.
	pub coverage_softness: f32,
	/// PBR perceptual roughness.
	pub roughness: f32,
	/// Blend displaced geometric normals toward up.
	pub normal_soften: f32,
	/// Strength of the multi-scale bites relative to the broad density mask.
	pub cheese_amount: f32,
	/// Frequency multiplier for the multi-scale bite field.
	pub cheese_scale: f32,
	/// Frequency multiplier for static fragment-scale apparent height.
	pub fragment_height_frequency: f32,
	/// Apparent fragment-scale height amplitude in world units.
	pub fragment_height_amplitude: f32,
}

impl BumpOutStyle {
	pub const fn new(coverage_softness: f32, roughness: f32, normal_soften: f32) -> Self {
		Self {
			coverage_softness,
			roughness,
			normal_soften,
			cheese_amount: 0.75,
			cheese_scale: 1.0,
			fragment_height_frequency: 3.5,
			fragment_height_amplitude: 0.18,
		}
	}

	pub const fn with_cheese(mut self, amount: f32, scale: f32) -> Self {
		self.cheese_amount = amount;
		self.cheese_scale = scale;
		self
	}

	pub const fn with_fragment_height(mut self, frequency: f32, amplitude: f32) -> Self {
		self.fragment_height_frequency = frequency;
		self.fragment_height_amplitude = amplitude;
		self
	}

	pub const fn as_values(self) -> [f32; 7] {
		[
			self.coverage_softness,
			self.roughness,
			self.normal_soften,
			self.cheese_amount,
			self.cheese_scale,
			self.fragment_height_frequency,
			self.fragment_height_amplitude,
		]
	}

	pub fn apply_to(self, material_ref: MaterialRef) -> MaterialRef {
		material_ref.with_parameter(STYLE_PARAMETER, self.as_values())
	}

	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let Some(values) = material_ref.parameter(STYLE_PARAMETER) else {
			return Self::default();
		};
		let defaults = Self::default();
		Self {
			coverage_softness: values.first().copied().unwrap_or(defaults.coverage_softness),
			roughness: values.get(1).copied().unwrap_or(defaults.roughness),
			normal_soften: values.get(2).copied().unwrap_or(defaults.normal_soften),
			cheese_amount: values.get(3).copied().unwrap_or(defaults.cheese_amount),
			cheese_scale: values.get(4).copied().unwrap_or(defaults.cheese_scale),
			fragment_height_frequency: values
				.get(5)
				.copied()
				.unwrap_or(defaults.fragment_height_frequency),
			fragment_height_amplitude: values
				.get(6)
				.copied()
				.unwrap_or(defaults.fragment_height_amplitude),
		}
	}
}

impl Default for BumpOutStyle {
	fn default() -> Self {
		Self::new(0.04, 0.92, 0.25)
	}
}

fn samples_from_ref(
	material_ref: &MaterialRef,
	name: &str,
	default_value: f32,
) -> [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES] {
	let mut samples = [default_value; BUMP_OUT_NEIGHBORHOOD_SAMPLES];
	if let Some(values) = material_ref.parameter(name) {
		for (sample, value) in samples.iter_mut().zip(values.iter().copied()) {
			*sample = value;
		}
	}
	samples
}
