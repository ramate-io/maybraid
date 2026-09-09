//! Stream knobs and keep-region driver. Playgrounds call [`TerrainDetailStreamLod::apply_spec`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lod::gen::{LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion};
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion};

use crate::{
	FormationExtent, FormationKind, TerrainDetailGenerateBullseye, TerrainDetailIndex,
	TerrainDetailLodChan, TerrainDetailPresentBullseye, TerrainDetailPresenterState,
	TerrainOutcropping, DEFAULT_OUTCROPPING_EXTENT_XZ, TERRAIN_DETAIL_GENERATE_RADIUS_M,
	TERRAIN_DETAIL_PRESENT_RADIUS_M,
};
use procedural_common::NoiseParams;

/// Default present ring multiplier (`1` → 800 m present / 1600 m generate).
pub const DEFAULT_TERRAIN_DETAIL_STREAM_RADIUS: u32 = 1;

/// Formation-throw default so neighboring 400 m cells stay related.
pub const DEFAULT_TERRAIN_DETAIL_NOISE: &str = "1337,0.0005,1,1";

/// Clap parser for a well-known formation kebab name.
pub fn parse_formation_kind(name: &str) -> Result<FormationKind, String> {
	FormationKind::from_kebab(name).ok_or_else(|| {
		let names: Vec<_> = FormationKind::ALL.iter().map(|kind| kind.as_kebab()).collect();
		format!("unknown formation {name:?}; expected one of: {}", names.join(", "))
	})
}

/// Live terrain-detail stream knobs (noise / ring / pinned formation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainDetailStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub formation: Option<FormationKind>,
}

impl Default for TerrainDetailStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_TERRAIN_DETAIL_STREAM_RADIUS,
			formation: None,
		}
	}
}

impl TerrainDetailStreamSpec {
	pub fn key(self) -> String {
		let formation_key = self.formation.map(FormationKind::as_kebab).unwrap_or("throw");
		format!("terrain-detail:{formation_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_OUTCROPPING_EXTENT_XZ, DEFAULT_OUTCROPPING_EXTENT_XZ * 2.0);
	}
	let present = TERRAIN_DETAIL_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (TERRAIN_DETAIL_GENERATE_RADIUS_M - TERRAIN_DETAIL_PRESENT_RADIUS_M))
}

/// Keep / queue / bullseye resources the stream system drives.
#[derive(SystemParam)]
pub struct TerrainDetailStreamLod<'w> {
	index: ResMut<'w, TerrainDetailIndex>,
	generate: ResMut<'w, TerrainDetailGenerateBullseye>,
	present: ResMut<'w, TerrainDetailPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<TerrainOutcropping>>,
	present_queue: ResMut<'w, LodPresentQueue<TerrainOutcropping>>,
	presenter: ResMut<'w, TerrainDetailPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<TerrainDetailLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<TerrainDetailLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<TerrainDetailLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<TerrainDetailLodChan>>,
}

impl TerrainDetailStreamLod<'_> {
	/// Enable or tear down the stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&TerrainDetailStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.index.clear();
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.clear();
			self.present_queue.clear();
			self.presenter.clear(commands);
			*last_key = Some(key);
		}

		self.index.noise = spec.noise;
		self.index.formation = spec.formation;
		let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
		self.generate.radius_m = generate_m;
		self.generate.enabled = true;
		self.present.radius_m = present_m;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let generate_aabb = FormationExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = FormationExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn default_noise_parses() -> Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_TERRAIN_DETAIL_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn parse_formation_kind_accepts_kebab() -> Result<()> {
		assert_eq!(
			parse_formation_kind("boulder-field").map_err(|e| anyhow::anyhow!("{e}"))?,
			FormationKind::BoulderField
		);
		assert!(parse_formation_kind("not-a-formation").is_err());
		Ok(())
	}

	#[test]
	fn default_stream_radii() -> Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_TERRAIN_DETAIL_STREAM_RADIUS);
		assert!((present - TERRAIN_DETAIL_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - TERRAIN_DETAIL_GENERATE_RADIUS_M).abs() < 1e-3);
		Ok(())
	}
}
