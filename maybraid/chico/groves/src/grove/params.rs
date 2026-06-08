//! Authored grove parameter ranges and per-cell placement sampling ([RFC-183 3.4.1]).

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use super::biases::ForestGroveBiases;

/// Authored min/max ranges owned by a grove definition.
///
/// `cell_size` and `density` are grove-level: the forest uses them when gridding and biasing
/// composition, not per vegetation cell during variant selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroveParamRanges {
	pub cell_size: UnitRange,
	pub scale: UnitRange,
	pub density: UnitRange,
	pub offset: UnitRange,
	pub noise_amplitude: UnitRange,
	pub noise_frequency: UnitRange,
}

impl GroveParamRanges {
	pub const fn new(
		cell_size: UnitRange,
		scale: UnitRange,
		density: UnitRange,
		offset: UnitRange,
		noise_amplitude: UnitRange,
		noise_frequency: UnitRange,
	) -> Self {
		Self { cell_size, scale, density, offset, noise_amplitude, noise_frequency }
	}
}

/// Per-cell placement sample: offset, instance scale, and noise params for this draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledCellParams {
	pub noise: NoiseParams,
	pub scale: f32,
	pub offset: Vec2,
}

/// Shared noise seed/configuration for grove sampling channels.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Grove Noise"))]
pub struct GroveNoiseConfig {
	#[cfg_attr(
		feature = "render",
		arg(
			long = "grove-noise",
			default_value = "1337,1,1,1",
			value_parser = procedural_common::noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		)
	)]
	pub base: NoiseParams,
}

impl Default for GroveNoiseConfig {
	fn default() -> Self {
		Self { base: NoiseParams::default() }
	}
}

impl GroveNoiseConfig {
	pub fn new(base: NoiseParams) -> Self {
		Self { base }
	}
}

/// Saturating scalar sample inside an authored range ([RFC-183 3.5.1.1]).
pub fn biased_sample(range: UnitRange, mean_unit: f32, noise: f32) -> f32 {
	let mean_unit = mean_unit.clamp(0.0, 1.0);
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	let mean = lo + (hi - lo) * mean_unit;
	let radius = (mean - lo).max(hi - mean);
	(mean + noise * radius).clamp(lo, hi)
}

/// Sample placement parameters for one vegetation cell inside a grove.
pub fn sample_cell_params(
	ranges: &GroveParamRanges,
	biases: &ForestGroveBiases,
	noise: &GroveNoiseConfig,
	sample_position: bevy_math::Vec3,
) -> SampledCellParams {
	let n = NoiseConfig::new(noise.base);
	let scale = biased_sample(
		ranges.scale,
		biases.scale_mean,
		n.sample_3d_world(sample_position + bevy_math::Vec3::new(2.0, 0.0, 0.0)),
	);
	let amplitude = biased_sample(
		ranges.noise_amplitude,
		biases.noise_amplitude_mean,
		n.sample_3d_world(sample_position + bevy_math::Vec3::new(4.0, 0.0, 0.0)),
	);
	let frequency = biased_sample(
		ranges.noise_frequency,
		biases.noise_frequency_mean,
		n.sample_3d_world(sample_position + bevy_math::Vec3::new(5.0, 0.0, 0.0)),
	);
	let offset_x = biased_sample(
		ranges.offset,
		biases.offset_mean,
		n.sample_2d_world(sample_position.truncate() + Vec2::new(6.0, 0.0)),
	);
	let offset_y = biased_sample(
		ranges.offset,
		biases.offset_mean,
		n.sample_2d_world(sample_position.truncate() + Vec2::new(0.0, 7.0)),
	);
	SampledCellParams {
		noise: NoiseParams { amplitude, frequency, ..noise.base },
		scale,
		offset: Vec2::new(offset_x, offset_y),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn biased_sample_stays_in_range() -> Result<()> {
		let range = UnitRange::new(0.2, 0.8);
		for noise in [-1.0_f32, 0.0, 1.0] {
			let v = biased_sample(range, 0.5, noise);
			assert!(v >= 0.2 && v <= 0.8);
		}
		Ok(())
	}

	#[test]
	fn sample_cell_params_is_deterministic() -> Result<()> {
		let ranges = GroveParamRanges::new(
			UnitRange::new(8.0, 16.0),
			UnitRange::new(0.8, 1.2),
			UnitRange::new(0.1, 0.5),
			UnitRange::new(0.0, 0.2),
			UnitRange::new(0.02, 0.12),
			UnitRange::new(0.01, 0.03),
		);
		let pos = bevy_math::Vec3::new(10.0, 0.0, 20.0);
		let a = sample_cell_params(&ranges, &ForestGroveBiases::default(), &GroveNoiseConfig::default(), pos);
		let b = sample_cell_params(&ranges, &ForestGroveBiases::default(), &GroveNoiseConfig::default(), pos);
		assert_eq!(a, b);
		Ok(())
	}
}
