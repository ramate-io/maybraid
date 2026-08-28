//! `/show forest` glue: enable the independent generate / present / cull plugins.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_forests::{
	forest_world_sample, match_forest_grove_tile, ChicoGrove, ForestExtent, ForestGenerateBullseye,
	ForestGroveTile, ForestIndex, ForestLodChan, ForestPresentBullseye, LayeringKind,
	DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
};
use chico_vegetation_components::{
	spawn_lod_scene_host_with_lod_ref, vegetation_bounds, VegetationComponents,
};
use lod::gen::{Id, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion, LodScene, Version};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion, RegionPresenter};

use crate::camera::CameraController;
use crate::commands::show::{ShowConfig, ShowRoot, ShowSubject};
use crate::ground::GroundPlane;

/// Default present ring multiplier (`1` → 1 km present / 2 km generate).
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

const FOREST_CAMERA_SPEED: f32 = 80.0;
const DEFAULT_CAMERA_SPEED: f32 = 18.0;

#[derive(Resource, Default)]
pub struct ForestPresenterState {
	presented: HashMap<Id, PresentedGrove>,
}

struct PresentedGrove {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

impl ForestPresenterState {
	fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
	}
}

/// Spawns grove [`LodScene`] hosts for a generated [`ChicoGrove`].
#[derive(SystemParam)]
pub struct ForestRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
}

fn spawn_grove_host<T>(commands: &mut Commands, grove: &T, lod_ref: &LodRef) -> Vec<Entity>
where
	T: LodScene + VegetationComponents + Component + Clone + Send + Sync + 'static,
{
	let bounds = grove
		.structural_lod()
		.map(|p| p.footprint_aabb())
		.unwrap_or_else(|| vegetation_bounds(grove));
	let entities =
		spawn_lod_scene_host_with_lod_ref(commands, grove, Transform::IDENTITY, bounds, lod_ref);
	for entity in &entities {
		commands.entity(*entity).insert(ShowRoot);
	}
	entities
}

fn spawn_forest_grove_tile(
	commands: &mut Commands,
	tile: &ForestGroveTile,
	lod_ref: &LodRef,
) -> Vec<Entity> {
	match_forest_grove_tile!(tile, g => spawn_grove_host(commands, g, lod_ref))
}

impl RegionPresenter<ChicoGrove, ForestIndex> for ForestRegionPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|entry| entry.version)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		if let Some(previous) = self.state.presented.remove(&id) {
			for entity in previous.entities {
				self.commands.entity(entity).despawn();
			}
		}
		let world = forest_world_sample();
		let mut entities = Vec::new();
		for tile in grove.grow(&world) {
			entities.extend(spawn_forest_grove_tile(&mut self.commands, &tile, lod_ref));
		}
		self.state
			.presented
			.insert(id, PresentedGrove { version, entities, hidden: false });
	}

	fn hide(&mut self, id: Id) {
		if let Some(entry) = self.state.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
		}
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.state.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.state.presented.remove(&id) {
				for entity in entry.entities {
					self.commands.entity(entity).despawn();
				}
			}
		}
	}
}

#[derive(SystemParam)]
pub(crate) struct ForestStreamLod<'w> {
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

/// Keep camera / ground / subject key in sync and bootstrap forest regions.
pub fn stream_forest(
	mut commands: Commands,
	config: Res<ShowConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut controller: Query<&mut CameraController, With<Camera3d>>,
	mut ground: Query<&mut Transform, (With<GroundPlane>, Without<Camera3d>)>,
	mut lod: ForestStreamLod,
	mut last_key: Local<Option<String>>,
	mut forest_camera: Local<bool>,
) {
	let Some(ShowSubject::Forest { noise, stream_radius, layering }) = &config.subject else {
		if *forest_camera {
			if let Ok(mut ctrl) = controller.single_mut() {
				ctrl.speed = DEFAULT_CAMERA_SPEED;
			}
			*forest_camera = false;
		}
		lod.generate.enabled = false;
		lod.present.enabled = false;
		lod.generate_keep.region = None;
		lod.keep.region = None;
		lod.index.clear();
		lod.generate_queue.pending.clear();
		lod.present_queue.pending.clear();
		lod.presenter.clear(&mut commands);
		last_key.take();
		return;
	};

	if !*forest_camera {
		if let Ok(mut ctrl) = controller.single_mut() {
			ctrl.speed = FOREST_CAMERA_SPEED;
		}
		*forest_camera = true;
	}

	let layering_key = layering.map(LayeringKind::as_kebab).unwrap_or("hopscotch");
	let key = format!("forest:{layering_key}|{noise:?}|r={stream_radius}");
	let key_changed = last_key.as_ref() != Some(&key);
	if key_changed {
		lod.index.clear();
		lod.generate_queue.pending.clear();
		lod.present_queue.pending.clear();
		lod.presenter.clear(&mut commands);
		*last_key = Some(key);
	}

	lod.index.noise = *noise;
	lod.index.layering = *layering;
	let (present_m, generate_m) = stream_radii_m(*stream_radius);
	lod.generate.radius_m = generate_m;
	lod.generate.enabled = true;
	lod.present.radius_m = present_m;
	lod.present.enabled = true;

	let Ok(cam) = camera.single() else {
		return;
	};
	if let Ok(mut ground_tf) = ground.single_mut() {
		ground_tf.translation.x = cam.translation.x;
		ground_tf.translation.z = cam.translation.z;
	}

	let generate_aabb = ForestExtent::xz_radius_aabb(cam.translation, generate_m);
	let present_aabb = ForestExtent::xz_radius_aabb(cam.translation, present_m);
	lod.generate_keep.region = Some(generate_aabb);
	lod.keep.region = Some(present_aabb);
	if key_changed {
		lod.generate_regions.write(LodGenerateRegion::new(generate_aabb));
		lod.present_regions.write(LodPresentRegion::new(present_aabb));
	}
}

fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
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
	fn default_stream_radii_are_one_and_two_kilometres() -> Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_FOREST_STREAM_RADIUS);
		assert!((present - GROVE_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - GROVE_GENERATE_RADIUS_M).abs() < 1e-3);
		let (tight_present, tight_generate) = stream_radii_m(0);
		assert!((tight_present - DEFAULT_FOREST_GROVE_TILE_XZ).abs() < 1e-3);
		assert!(tight_generate > tight_present);
		Ok(())
	}
}
