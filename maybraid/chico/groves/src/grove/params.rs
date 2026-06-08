//! Per-cell placement sampling ranges ([RFC-183 3.4.1]).

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use super::biases::ForestGroveBiases;
use super::placement::CellXzOffset;

/// Authored ranges for parameters sampled **inside** each vegetation cell during selection.
///
/// Cell grid footprint ([`super::CellGrove::cell_extent_xz`]) and fill density
/// ([`super::GroveDistribution`] bucket weights, including `None`) are owned elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrovePlacementRanges {
	/// The scale range of the vegetation instances.
	pub scale: UnitRange,
	/// Signed world-metre shift on each horizontal axis from the cell center (usually symmetric).
	pub offset: UnitRange,
	/// The amplitude range of the noise passed down to the vegetation instances.
	pub noise_amplitude: UnitRange,
	/// The frequency range of the noise passed down to the vegetation instances.
	pub noise_frequency: UnitRange,
}

impl GrovePlacementRanges {
	pub const fn new(
		scale: UnitRange,
		offset: UnitRange,
		noise_amplitude: UnitRange,
		noise_frequency: UnitRange,
	) -> Self {
		Self { scale, offset, noise_amplitude, noise_frequency }
	}
}

/// Per-cell placement sample: horizontal shift, instance scale, and foliage noise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledCellParams {
	pub noise: NoiseParams,
	pub scale: f32,
	pub offset: CellXzOffset,
}

impl SampledCellParams {
	/// Sample placement parameters for one vegetation cell inside a grove.
	///
	/// `cell_center` should be the parent cell center in world space (see [`super::placement::cell_origin`]).
	pub fn sample(
		ranges: &GrovePlacementRanges,
		biases: &ForestGroveBiases,
		noise: &GroveNoiseConfig,
		cell_center: Vec3,
	) -> Self {
		let n = NoiseConfig::new(noise.base);
		let scale = biased_sample(
			ranges.scale,
			biases.scale_mean,
			n.sample_3d_world(cell_center + Vec3::new(2.0, 0.0, 0.0)),
		);
		let amplitude = biased_sample(
			ranges.noise_amplitude,
			biases.noise_amplitude_mean,
			n.sample_3d_world(cell_center + Vec3::new(4.0, 0.0, 0.0)),
		);
		let frequency = biased_sample(
			ranges.noise_frequency,
			biases.noise_frequency_mean,
			n.sample_3d_world(cell_center + Vec3::new(5.0, 0.0, 0.0)),
		);
		let offset_x = biased_sample(
			ranges.offset,
			biases.offset_mean,
			n.sample_3d_world(cell_center + Vec3::new(6.0, 0.0, 0.0)),
		);
		let offset_z = biased_sample(
			ranges.offset,
			biases.offset_mean,
			n.sample_3d_world(cell_center + Vec3::new(0.0, 0.0, 7.0)),
		);
		Self {
			noise: NoiseParams { amplitude, frequency, ..noise.base },
			scale,
			offset: CellXzOffset::new(offset_x, offset_z),
		}
	}
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

/// [`SampledCellParams::sample`] alias kept for older call sites.
pub fn sample_cell_params(
	ranges: &GrovePlacementRanges,
	biases: &ForestGroveBiases,
	noise: &GroveNoiseConfig,
	cell_center: Vec3,
) -> SampledCellParams {
	SampledCellParams::sample(ranges, biases, noise, cell_center)
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
	fn sample_is_deterministic_for_cell_center() -> Result<()> {
		let ranges = GrovePlacementRanges::new(
			UnitRange::new(0.8, 1.2),
			UnitRange::new(-0.2, 0.2),
			UnitRange::new(0.02, 0.12),
			UnitRange::new(0.01, 0.03),
		);
		let pos = Vec3::new(10.0, 0.0, 20.0);
		let biases = ForestGroveBiases::default();
		let noise = GroveNoiseConfig::default();
		let a = SampledCellParams::sample(&ranges, &biases, &noise, pos);
		let b = SampledCellParams::sample(&ranges, &biases, &noise, pos);
		assert_eq!(a, b);
		Ok(())
	}

	#[test]
	fn offset_varies_with_cell_center_z() -> Result<()> {
		let ranges = GrovePlacementRanges::new(
			UnitRange::new(1.0, 1.0),
			UnitRange::new(-1.0, 1.0),
			UnitRange::new(0.1, 0.1),
			UnitRange::new(0.05, 0.05),
		);
		let biases = ForestGroveBiases::default();
		let noise = GroveNoiseConfig::default();
		let along_x = SampledCellParams::sample(&ranges, &biases, &noise, Vec3::new(2.0, 0.5, 4.0));
		let along_z =
			SampledCellParams::sample(&ranges, &biases, &noise, Vec3::new(2.0, 0.5, 24.0));
		assert_ne!(
			along_x.offset, along_z.offset,
			"offset should vary when the cell center moves on Z"
		);
		Ok(())
	}
}
