//! Stamp forest noise / layering and enable grove bullseyes for [`ForestPlugin`].
//!
//! Keep is armed by generate / present produce from the lattice disk. Do not
//! emit a full-ring [`LodGenerateRegion`] / [`LodPresentRegion`] on install.

use bevy::prelude::*;
use procedural_common::NoiseParams;

use crate::{
	ForestGenerateBullseye, ForestIndex, ForestPresentBullseye, LayeringKind,
	DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
};

/// Default present ring multiplier (`1` → 1 km grove present / 3 km generate).
pub const DEFAULT_FOREST_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_FOREST_NOISE: &str = "1337,0.0005,1,1";

/// Clap parser for a well-known layering kebab name.
pub fn parse_layering_kind(name: &str) -> Result<LayeringKind, String> {
	LayeringKind::from_kebab(name).ok_or_else(|| {
		let names: Vec<_> = LayeringKind::ALL.iter().map(|kind| kind.as_kebab()).collect();
		format!("unknown layering {name:?}; expected one of: {}", names.join(", "))
	})
}

/// Live forest-stream knobs (noise / ring / pinned layering).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
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

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
}

/// Stamp [`ForestIndex`] noise / layering and enable grove bullseyes.
///
/// Call after [`crate::ForestPlugin`] so the bullseye resources exist. Does not
/// write a generate / present region impulse; produce arms keep from the
/// current lattice disk and drain scans it once.
pub fn install_forest_stream(app: &mut App, spec: ForestStreamSpec) {
	let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
	{
		let mut index = app.world_mut().resource_mut::<ForestIndex>();
		index.noise = spec.noise;
		index.layering = spec.layering;
	}
	{
		let mut generate = app.world_mut().resource_mut::<ForestGenerateBullseye>();
		generate.radius_m = generate_m;
		generate.enabled = true;
	}
	{
		let mut present = app.world_mut().resource_mut::<ForestPresentBullseye>();
		present.radius_m = present_m;
		present.enabled = true;
	}
	app.insert_resource(spec);
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn default_forest_noise_parses() -> Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn parse_layering_kind_accepts_kebab() -> Result<()> {
		assert_eq!(
			parse_layering_kind("ag-town").map_err(|e| anyhow::anyhow!("{e}"))?,
			LayeringKind::AgTown
		);
		assert!(parse_layering_kind("not-a-forest").is_err());
		Ok(())
	}

	#[test]
	fn default_stream_radii_are_one_and_three_kilometres() -> Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_FOREST_STREAM_RADIUS);
		assert!((present - GROVE_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - GROVE_GENERATE_RADIUS_M).abs() < 1e-3);
		let (tight_present, tight_generate) = stream_radii_m(0);
		assert!((tight_present - DEFAULT_FOREST_GROVE_TILE_XZ).abs() < 1e-3);
		assert!(tight_generate > tight_present);
		Ok(())
	}

	#[test]
	fn default_spec_matches_noise_string() -> Result<()> {
		let parsed = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let spec = ForestStreamSpec::default();
		assert_eq!(spec.noise.seed, parsed.seed);
		assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
		assert_eq!(spec.stream_radius, DEFAULT_FOREST_STREAM_RADIUS);
		Ok(())
	}
}
