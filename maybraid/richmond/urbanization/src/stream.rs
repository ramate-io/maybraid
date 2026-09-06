//! Stamp urbanization noise / kind and enable the generate bullseye.
//!
//! Keep is armed by generate produce from the lattice disk. Do not emit a
//! full-ring [`lod::LodGenerateRegion`] on install. Present stays disabled
//! until an urbanization present plugin enables it
//! ([#720](https://github.com/ramate-io/maybraid/issues/720) step 3).

use bevy::prelude::*;
use procedural_common::NoiseParams;

use crate::{
	UrbanizationGenerateBullseye, UrbanizationIndex, UrbanizationKind,
	DEFAULT_URBANIZATION_EXTENT_XZ, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

/// Default present ring multiplier (`1` → 1 km present / 3 km generate).
pub const DEFAULT_URBANIZATION_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_URBANIZATION_NOISE: &str = "1337,0.0005,1,1";

/// Clap parser for a well-known urbanization kebab name.
pub fn parse_urbanization_kind(name: &str) -> Result<UrbanizationKind, String> {
	UrbanizationKind::from_kebab(name).ok_or_else(|| {
		let names: Vec<_> = UrbanizationKind::ALL.iter().map(|kind| kind.as_kebab()).collect();
		format!("unknown urbanization {name:?}; expected one of: {}", names.join(", "))
	})
}

/// Live urbanization-stream knobs (noise / ring / pinned kind).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct UrbanizationStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub kind: Option<UrbanizationKind>,
}

impl Default for UrbanizationStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_URBANIZATION_STREAM_RADIUS,
			kind: None,
		}
	}
}

impl UrbanizationStreamSpec {
	pub fn key(self) -> String {
		let kind_key = self.kind.map(UrbanizationKind::as_kebab).unwrap_or("hopscotch");
		format!("urbanization:{kind_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_URBANIZATION_EXTENT_XZ, DEFAULT_URBANIZATION_EXTENT_XZ * 2.0);
	}
	let present = DEVELOPMENT_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (DEVELOPMENT_GENERATE_RADIUS_M - DEVELOPMENT_PRESENT_RADIUS_M))
}

/// Stamp [`UrbanizationIndex`] noise / kind and enable the generate bullseye.
///
/// Call after [`crate::UrbanizationGenerationPlugin`] so the bullseye exists.
/// Does not write a generate region impulse; produce arms keep from the current
/// lattice disk. Does not enable present.
pub fn install_urbanization_generate_stream(app: &mut App, spec: UrbanizationStreamSpec) {
	let (_present_m, generate_m) = stream_radii_m(spec.stream_radius);
	{
		let mut index = app.world_mut().resource_mut::<UrbanizationIndex>();
		index.noise = spec.noise;
		index.kind = spec.kind;
	}
	{
		let mut generate = app.world_mut().resource_mut::<UrbanizationGenerateBullseye>();
		generate.radius_m = generate_m;
		generate.enabled = true;
	}
	app.insert_resource(spec);
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn default_urbanization_noise_parses() -> Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn parse_urbanization_kind_accepts_kebab() -> Result<()> {
		assert_eq!(
			parse_urbanization_kind("frontier").map_err(|e| anyhow::anyhow!("{e}"))?,
			UrbanizationKind::Frontier
		);
		assert!(parse_urbanization_kind("not-a-city").is_err());
		Ok(())
	}

	#[test]
	fn default_stream_radii_are_one_and_three_kilometres() -> Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_URBANIZATION_STREAM_RADIUS);
		assert!((present - DEVELOPMENT_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn default_spec_matches_noise_string() -> Result<()> {
		let parsed = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let spec = UrbanizationStreamSpec::default();
		assert_eq!(spec.noise.seed, parsed.seed);
		assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
		assert_eq!(spec.stream_radius, DEFAULT_URBANIZATION_STREAM_RADIUS);
		Ok(())
	}

	#[test]
	fn install_enables_generate_not_present() {
		let mut app = App::new();
		app.init_resource::<UrbanizationIndex>()
			.init_resource::<UrbanizationGenerateBullseye>()
			.init_resource::<crate::UrbanizationPresentBullseye>();
		install_urbanization_generate_stream(&mut app, UrbanizationStreamSpec::default());
		let generate = app.world().resource::<UrbanizationGenerateBullseye>();
		assert!(generate.enabled);
		assert!((generate.radius_m - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
		let present = app.world().resource::<crate::UrbanizationPresentBullseye>();
		assert!(!present.enabled);
	}
}
