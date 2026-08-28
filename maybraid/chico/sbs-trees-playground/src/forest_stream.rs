//! Forest generate / present / cull glue. `/show forest` and other playgrounds
//! share the plugins; only the grow sample differs.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_forests::{
	forest_world_sample, match_forest_grove_tile, ChicoGrove, ForestExtent, ForestGenerateBullseye,
	ForestGroveTile, ForestIndex, ForestLodChan, ForestPresentBullseye, LayeringKind,
	DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
};
use chico_groves::GroveWorldSample;
use chico_vegetation_components::{
	spawn_lod_scene_host_with_lod_ref, vegetation_bounds, VegetationComponents,
};
use lod::gen::{
	Id, LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion, LodScene,
	SpatialIndex, Version,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion, RegionPresenter};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use lod_visual_pbr::{attach_forest_grove_visual, ForestGroveVisualPlugin};
use procedural_common::NoiseParams;

use crate::camera::CameraController;
use crate::commands::show::{ShowConfig, ShowRoot, ShowSubject};
use crate::ground::GroundPlane;

/// Default present ring multiplier (`1` → 1 km present / 2 km generate).
pub const DEFAULT_FOREST_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_FOREST_NOISE: &str = "1337,0.0005,1,1";

pub const FOREST_CAMERA_SPEED: f32 = 80.0;
const DEFAULT_CAMERA_SPEED: f32 = 18.0;

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

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
}

/// Independent generate / present / cull plugins. `Pr` grows with the playground
/// sample (flat in SBS, Durham on vegetation-on-terrain / world).
pub fn register_forest_lod<Pr>(app: &mut App)
where
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<ChicoGrove, ForestIndex>,
{
	if !app.is_plugin_added::<ForestGroveVisualPlugin>() {
		app.add_plugins(ForestGroveVisualPlugin);
	}
	app.init_resource::<ForestIndex>()
		.init_resource::<ForestPresenterState>()
		.insert_resource(LodGenerateBudget { ids_per_frame: 16 })
		.add_plugins(LodGenerateRegionPlugin::<
			ForestGenerateBullseye,
			With<LodViewer>,
			ForestLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			ChicoGrove,
			ForestIndex,
			ForestLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			ForestPresentBullseye,
			With<LodViewer>,
			ForestLodChan,
		>::default())
		.add_plugins(LodPresentPlugin::<
			ChicoGrove,
			ForestIndex,
			Pr,
			ForestLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			ChicoGrove,
			ForestIndex,
			Pr,
			ForestLodChan,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

#[derive(Resource, Default)]
pub struct ForestPresenterState {
	presented: HashMap<Id, PresentedGrove>,
	/// Replaced hosts waiting for the present-cull despawn budget. FIFO batches
	/// (one prior grove's entities per slot). `handle` never despawns.
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedGrove {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

impl ForestPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
		for entities in self.pending_despawn.drain(..) {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	fn retire(&mut self, id: Id) -> Option<PresentedGrove> {
		self.presented.remove(&id)
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn hide(&mut self, commands: &mut Commands, id: Id) {
		if let Some(entry) = self.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				commands.entity(*entity).insert(Visibility::Hidden);
			}
		}
	}

	pub fn is_hidden(&self, id: Id) -> bool {
		self.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	pub fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	pub fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				for entity in entry.entities {
					commands.entity(entity).despawn();
				}
			}
		}
	}

	/// Grow (or spawn) with `world`. Returns hosts spawned this slot (empty when
	/// this call only grew).
	pub fn present_with_world(
		&mut self,
		commands: &mut Commands,
		id: Id,
		version: Version,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
		world: &impl GroveWorldSample,
	) -> Vec<Entity> {
		if let Some(previous) = self.retire(id) {
			for entity in &previous.entities {
				commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.pending_despawn.push_back(previous.entities);
		}
		let Some(tiles) = grove.tiles_ready_to_present(world) else {
			return Vec::new();
		};
		let mut entities = Vec::new();
		for tile in tiles {
			entities.extend(spawn_forest_grove_tile(commands, tile, lod_ref));
		}
		self.presented
			.insert(id, PresentedGrove { version, entities: entities.clone(), hidden: false });
		entities
	}

	pub fn cull(
		&mut self,
		commands: &mut Commands,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		mut despawn_budget: u32,
	) -> u32 {
		while despawn_budget > 0 {
			let Some(entities) = self.pending_despawn.pop_front() else {
				break;
			};
			for entity in entities {
				commands.entity(entity).despawn();
			}
			despawn_budget -= 1;
		}
		let stale: Vec<Id> = self
			.presented_ids()
			.into_iter()
			.filter(|id| !keep.contains(id))
			.filter(|id| SpatialIndex::<ChicoGrove>::get_bounds(spatial_index, *id).is_some())
			.collect();
		let mut to_remove = HashSet::new();
		for id in stale {
			if !self.is_hidden(id) {
				self.hide(commands, id);
			}
			if despawn_budget > 0 {
				to_remove.insert(id);
				despawn_budget -= 1;
			}
		}
		if !to_remove.is_empty() {
			let wanted: HashSet<Id> =
				self.presented_ids().into_iter().filter(|id| !to_remove.contains(&id)).collect();
			self.remove_stale(commands, &wanted);
		}
		despawn_budget
	}
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
		attach_forest_grove_visual(commands, *entity, bounds);
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

/// Flat-ground presenter for the SBS trees playground.
#[derive(SystemParam)]
pub struct ForestRegionPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
}

impl RegionPresenter<ChicoGrove, ForestIndex> for ForestRegionPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		let entities = self.state.present_with_world(
			&mut self.commands,
			id,
			version,
			grove,
			lod_ref,
			&forest_world_sample(),
		);
		for entity in entities {
			self.commands.entity(entity).insert(ShowRoot);
		}
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}

	fn cull(
		&mut self,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
	}
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
			self.generate_queue.pending.clear();
			self.present_queue.pending.clear();
			self.presenter.clear(commands);
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.pending.clear();
			self.present_queue.pending.clear();
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
		let generate_aabb = ForestExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = ForestExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
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
	let spec = match &config.subject {
		Some(ShowSubject::Forest { noise, stream_radius, layering }) => Some(ForestStreamSpec {
			noise: *noise,
			stream_radius: *stream_radius,
			layering: *layering,
		}),
		_ => None,
	};

	if spec.is_some() != *forest_camera {
		if let Ok(mut ctrl) = controller.single_mut() {
			ctrl.speed = if spec.is_some() { FOREST_CAMERA_SPEED } else { DEFAULT_CAMERA_SPEED };
		}
		*forest_camera = spec.is_some();
	}

	if let (Some(_), Ok(cam)) = (spec, camera.single()) {
		if let Ok(mut ground_tf) = ground.single_mut() {
			ground_tf.translation.x = cam.translation.x;
			ground_tf.translation.z = cam.translation.z;
		}
	}

	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, spec.as_ref(), cam, &mut last_key);
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

	#[test]
	fn retire_queues_previous_hosts_without_dropping_them() -> Result<()> {
		use bevy::math::bounding::Aabb3d;
		use bevy::math::Vec3;

		let mut state = ForestPresenterState::default();
		let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let entity = Entity::from_raw_u32(7).expect("test entity");
		state.presented.insert(
			id,
			PresentedGrove { version: Version(1), entities: vec![entity], hidden: false },
		);
		let previous = state.retire(id).ok_or_else(|| anyhow::anyhow!("retired"))?;
		assert!(state.presented.is_empty());
		state.pending_despawn.push_back(previous.entities);
		assert_eq!(state.pending_despawn.len(), 1);
		assert_eq!(state.pending_despawn[0], vec![entity]);
		Ok(())
	}
}
