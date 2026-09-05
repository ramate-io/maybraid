//! Camera-driven generate / present keep regions for [`ForestPlugin`](crate::ForestPlugin).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use procedural_common::NoiseParams;

use crate::{
	ChicoGrove, ForestGenerateBullseye, ForestIndex, ForestLodChan, ForestPresentBullseye,
	ForestPresenterState, LayeringKind, DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M,
	GROVE_PRESENT_RADIUS_M,
};
use lod::gen::{LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion};
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion};
use lod::LodViewer;

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

/// Optional active forest stream. `None` tears generate / present down.
#[derive(Resource, Clone, Default)]
pub struct ForestStream(pub Option<ForestStreamSpec>);

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
}

/// Keep / queue / bullseye resources the stream system drives.
#[derive(SystemParam)]
pub struct ForestStreamLod<'w> {
	index: ResMut<'w, ForestIndex>,
	generate: ResMut<'w, ForestGenerateBullseye>,
	present: ResMut<'w, ForestPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<ChicoGrove>>,
	present_queue: ResMut<'w, LodPresentQueue<ChicoGrove>>,
	presenter: ResMut<'w, ForestPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<ForestLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<ForestLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<ForestLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<ForestLodChan>>,
}

impl ForestStreamLod<'_> {
	/// Enable or tear down the forest stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&ForestStreamSpec>,
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
		self.index.layering = spec.layering;
		let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
		self.generate.radius_m = generate_m;
		self.generate.enabled = true;
		self.present.radius_m = present_m;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let generate_aabb = crate::ForestExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = crate::ForestExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
}

/// Keep generate / present queues in sync with [`ForestStream`] and the viewer.
pub fn drive_forest_stream(
	mut commands: Commands,
	stream: Res<ForestStream>,
	camera: Query<&Transform, With<LodViewer>>,
	mut lod: ForestStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, stream.0.as_ref(), cam, &mut last_key);
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
