//! Layer knobs the vegetation playground's `PlaygroundConfig` used to carry.

use bevy::prelude::*;
use chico::LayeringKind;
use procedural_common::NoiseParams;

use crate::stream::DEFAULT_FOREST_STREAM_RADIUS;

/// Live forest-stream knobs (noise / ring / pinned layering).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForestStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub layering: Option<LayeringKind>,
}

impl Default for ForestStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_FOREST_STREAM_RADIUS,
			layering: None,
		}
	}
}

impl ForestStreamSpec {
	pub fn key(self) -> String {
		let layering_key = self.layering.map(LayeringKind::as_kebab).unwrap_or("hopscotch");
		format!("forest:{layering_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// Stream spec and the three vegetation generate budgets.
///
/// Every budget is 16 in both the world and the playground. [`Self::world_defaults`]
/// arms the forest at stream radius 1 (1 km present / 3 km generate).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct VegetationLayerConfig {
	pub forest: Option<ForestStreamSpec>,
	pub forest_budget: u32,
	pub bump_out_budget: u32,
	pub medium_bump_out_budget: u32,
}

impl Default for VegetationLayerConfig {
	fn default() -> Self {
		Self {
			forest: None,
			forest_budget: 16,
			bump_out_budget: 16,
			medium_bump_out_budget: 16,
		}
	}
}

impl VegetationLayerConfig {
	/// Forest on at radius 1, generate budgets 16.
	pub fn world_defaults() -> Self {
		Self {
			forest: Some(ForestStreamSpec { stream_radius: 1, ..ForestStreamSpec::default() }),
			..Self::default()
		}
	}

	/// Forest on at radius 0 (one grove extent), generate budgets 16.
	pub fn grove() -> Self {
		Self {
			forest: Some(ForestStreamSpec { stream_radius: 0, ..ForestStreamSpec::default() }),
			..Self::world_defaults()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stream::{stream_radii_m, DEFAULT_FOREST_NOISE};
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn world_defaults_keep_grove_fill_at_one_kilometre() -> anyhow::Result<()> {
		let spec = VegetationLayerConfig::world_defaults()
			.forest
			.ok_or_else(|| anyhow::anyhow!("forest on"))?;
		assert_eq!(spec.stream_radius, 1);
		assert_eq!(stream_radii_m(1), (1_000.0, 3_000.0));
		assert_eq!(VegetationLayerConfig::default().forest_budget, 16);
		assert_eq!(VegetationLayerConfig::default().bump_out_budget, 16);
		assert_eq!(VegetationLayerConfig::default().medium_bump_out_budget, 16);
		assert_eq!(VegetationLayerConfig::world_defaults().forest_budget, 16);
		Ok(())
	}

	#[test]
	fn grove_keeps_one_extent() -> anyhow::Result<()> {
		let spec = VegetationLayerConfig::grove()
			.forest
			.ok_or_else(|| anyhow::anyhow!("forest on"))?;
		anyhow::ensure!(spec.stream_radius == 0, "grove stream radius is 0");
		anyhow::ensure!(
			VegetationLayerConfig::grove().forest_budget
				== VegetationLayerConfig::world_defaults().forest_budget,
			"grove keeps the same budgets"
		);
		Ok(())
	}

	#[test]
	fn default_forest_noise_parses() -> anyhow::Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn default_spec_matches_noise_string() -> anyhow::Result<()> {
		let parsed = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let spec = ForestStreamSpec::default();
		assert_eq!(spec.noise.seed, parsed.seed);
		assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
		assert_eq!(spec.stream_radius, DEFAULT_FOREST_STREAM_RADIUS);
		Ok(())
	}
}
