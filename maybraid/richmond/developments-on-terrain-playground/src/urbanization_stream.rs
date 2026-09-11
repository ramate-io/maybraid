//! Urbanization generate / host glue (forest_stream parallel).
//!
//! Registers LOD generate for [`SelectedUrbanization`] and a present-keep
//! bullseye. Host spawn / cull runs in Update so it can own
//! [`DevelopmentIndex`] without conflicting with the urbanization LOD drain.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham_terrain_models::{PresentedTerrainScene, TerrainCellLayout};
use lod::gen::{
	GeneratingSpatialIndex, Id, LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, MaterializeStatus, SpatialIndex, StorageStatus, Version,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::{LodGeneratePlugin, LodGenerateRegionPlugin, LodPresentRegionPlugin, LodViewer};
use procedural_common::NoiseParams;
use richmond_development_models::{
	BuiltDevelopment, DevelopmentCell, DevelopmentEntryStore, DevelopmentHosts, DevelopmentIndex,
	PaddedStoreView, PaddedTerrainPresenter, TerrainWithPads,
};
use richmond_urbanization::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationIndex, UrbanizationKind, UrbanizationLodChan, UrbanizationPresentBullseye,
	DEFAULT_URBANIZATION_EXTENT_XZ, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

use crate::hosts::DevelopmentHostRoot;

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
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// Generate + present-keep plugins for [`SelectedUrbanization`].
pub fn register_urbanization_lod(app: &mut App) {
	app.init_resource::<UrbanizationIndex>()
		.init_resource::<UrbanizationPresenterState>()
		.init_resource::<UrbanizationGenerateBullseye>()
		.init_resource::<UrbanizationPresentBullseye>()
		.insert_resource(LodGenerateBudget { ids_per_frame: 8 })
		.add_plugins(LodGenerateRegionPlugin::<
			UrbanizationGenerateBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			SelectedUrbanization,
			UrbanizationIndex,
			UrbanizationLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default());
}

#[derive(Resource, Default)]
pub struct UrbanizationPresenterState {
	presented: HashMap<Id, PresentedUrbanization>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

/// Padded terrain ids replacing raw Durham presentation roots this frame.
#[derive(Resource, Default)]
pub struct UrbanizationPaddedTerrainState {
	wanted: HashSet<Id>,
}

/// Type-erased anchor for one presented urban setting.
///
/// The world layer turns this into a global POI without coupling Richmond
/// generation or presentation to a particular intelligence implementation.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct UrbanSetting {
	pub id: Id,
	pub arrival_radius: f32,
}

struct PresentedUrbanization {
	version: Version,
	entities: Vec<Entity>,
}

impl UrbanizationPresenterState {
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

	fn retire(&mut self, id: Id) -> Option<PresentedUrbanization> {
		self.presented.remove(&id)
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	pub fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				self.pending_despawn.push_back(entry.entities);
			}
		}
		while let Some(entities) = self.pending_despawn.pop_front() {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	/// Spawn hosts for one filled leaf that already has [`BuiltDevelopment`].
	pub fn present_leaf(
		&mut self,
		commands: &mut Commands,
		leaf_id: Id,
		version: Version,
		cell: &DevelopmentCell,
		built: &BuiltDevelopment,
		leaf_bounds: bevy::math::bounding::Aabb3d,
	) {
		if self.presented_version(leaf_id) == Some(version) {
			return;
		}
		if let Some(previous) = self.retire(leaf_id) {
			self.pending_despawn.push_back(previous.entities);
		}

		let center = (leaf_bounds.min + leaf_bounds.max) * 0.5;
		let elevation = cell.pads().next().map(|pad| pad.height).unwrap_or(center.y);
		let arrival_radius = ((leaf_bounds.max.x - leaf_bounds.min.x)
			.min(leaf_bounds.max.z - leaf_bounds.min.z)
			* 0.25)
			.clamp(8.0, 128.0);
		let mut entities = vec![commands
			.spawn((
				Name::new("urban-setting"),
				UrbanSetting { id: leaf_id, arrival_radius },
				Transform::from_xyz(center.x, elevation, center.z),
			))
			.id()];
		entities.extend(spawn_tagged_hosts(commands, built));
		self.presented.insert(leaf_id, PresentedUrbanization { version, entities });
	}
}

fn spawn_tagged_hosts(commands: &mut Commands, development: &impl DevelopmentHosts) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		for entity in host.spawn(commands) {
			commands.entity(entity).insert(DevelopmentHostRoot);
			spawned.push(entity);
		}
	}
	spawned
}

/// Keep / queue / bullseye resources the stream system drives.
#[derive(SystemParam)]
pub struct UrbanizationStreamLod<'w> {
	index: ResMut<'w, UrbanizationIndex>,
	generate: ResMut<'w, UrbanizationGenerateBullseye>,
	present: ResMut<'w, UrbanizationPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<SelectedUrbanization>>,
	presenter: ResMut<'w, UrbanizationPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<UrbanizationLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<UrbanizationLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<UrbanizationLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<UrbanizationLodChan>>,
}

impl UrbanizationStreamLod<'_> {
	/// Enable or tear down the urbanization stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&UrbanizationStreamSpec>,
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
			self.presenter.clear(commands);
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.clear();
			self.presenter.clear(commands);
			*last_key = Some(key);
		}

		self.index.noise = spec.noise;
		self.index.kind = spec.kind;
		let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
		self.generate.radius_m = generate_m;
		self.generate.enabled = true;
		self.present.radius_m = present_m;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let generate_aabb = UrbanizationExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = UrbanizationExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
}

/// Drive urbanization bullseyes from [`crate::PlaygroundConfig::urbanization`].
pub fn stream_urbanization(
	mut commands: Commands,
	config: Res<crate::PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: UrbanizationStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, config.urbanization.as_ref(), cam, &mut last_key);
}

/// Bounded leaf generate on the 1 km urbanization keep. Height GET miss
/// leaves the leaf `NotTracked` so the next frame retries.
pub fn generate_urbanization_developments(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	mut development: DevelopmentIndex,
	budget: Res<LodGenerateBudget>,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	let noise = development.config().urbanization_noise();
	development.urbanization.noise = noise;
	if let Some(spec) = config.urbanization.as_ref() {
		development.urbanization.kind = spec.kind;
	}

	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let mut created = 0usize;
	let cap = budget.ids_per_frame.max(1) as usize;
	let urbanization_ids: Vec<Id> =
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*development.urbanization, region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect();
	for id in urbanization_ids {
		if created >= cap {
			break;
		}
		let Some(selected) = development.urbanization.get(id).cloned() else {
			continue;
		};
		for leaf in &selected.leaves {
			if created >= cap {
				break;
			}
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			if SpatialIndex::<DevelopmentCell>::storage_status(&development, leaf_id)
				== StorageStatus::NotTracked
			{
				if GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
					&mut development,
					leaf_id,
					&lod_ref,
				)
				.is_none()
				{
					continue;
				}
			}
			let Some(cell) = SpatialIndex::<DevelopmentCell>::get(&development, leaf_id) else {
				continue;
			};
			if !cell.is_filled() {
				continue;
			}
			if SpatialIndex::<BuiltDevelopment>::storage_status(&development, leaf_id)
				!= StorageStatus::NotTracked
			{
				continue;
			}
			if GeneratingSpatialIndex::<BuiltDevelopment>::get_or_generate(
				&mut development,
				leaf_id,
				&lod_ref,
			) == Some(MaterializeStatus::Created)
			{
				created += 1;
			}
		}
	}
}

/// GET-only host spawn for leaves that already have [`BuiltDevelopment`].
pub fn present_urbanization_hosts(
	mut commands: Commands,
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	development: DevelopmentIndex,
	mut state: ResMut<UrbanizationPresenterState>,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	let urbanization_ids: Vec<Id> =
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*development.urbanization, region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect();

	let mut wanted = HashSet::new();
	for id in urbanization_ids {
		let Some(selected) = development.urbanization.get(id) else {
			continue;
		};
		for leaf in &selected.leaves {
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			let Some(cell) = SpatialIndex::<DevelopmentCell>::get(&development, leaf_id) else {
				continue;
			};
			let Some(built) = SpatialIndex::<BuiltDevelopment>::get(&development, leaf_id) else {
				continue;
			};
			let Some(version) = SpatialIndex::<BuiltDevelopment>::version(&development, leaf_id)
			else {
				continue;
			};
			state.present_leaf(&mut commands, leaf_id, version, cell, built, leaf.bounds);
			wanted.insert(leaf_id);
		}
	}
	state.remove_stale(&mut commands, &wanted);
}

fn pad_visual_region(
	layout: &TerrainCellLayout,
	urban_keep: Option<bevy::math::bounding::Aabb3d>,
) -> Option<bevy::math::bounding::Aabb3d> {
	if layout.is_streamed() {
		Some(layout.presentation_region())
	} else {
		urban_keep
	}
}

/// Compose pads only for Durham cells that are already stored.
pub fn generate_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	layout: Res<TerrainCellLayout>,
	mut development: DevelopmentIndex,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = pad_visual_region(&layout, keep.region) else {
		return;
	};
	development.store.invalidate_dirty_padded();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	for id in development.terrain_store().terrain_ids_overlapping(region) {
		let _ = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate(
			&mut development,
			id,
			&lod_ref,
		);
	}
}

/// Present padded replacements for the urbanization keep and cull stale cells.
pub fn present_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	layout: Res<TerrainCellLayout>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
	state.wanted.clear();
	let Some(region) =
		pad_visual_region(&layout, keep.region).filter(|_| config.urbanization.is_some())
	else {
		presenter.remove_stale(&state.wanted);
		return;
	};
	let viewer = lod_viewers
		.iter()
		.next()
		.or_else(|| cameras.iter().next())
		.map(|tf| {
			let t = tf.translation();
			Transform::from_translation(Vec3::new(t.x, 0.0, t.z))
		})
		.unwrap_or(Transform::IDENTITY);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let view = PaddedStoreView::new(&store);
	presenter.present_banded(&view, region, &lod_ref);
	state.wanted = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
}

/// Hide raw Durham visual roots while their padded replacements are active.
/// Collision lives on the padded fill scene itself.
pub fn sync_raw_terrain_replacements(
	state: Res<UrbanizationPaddedTerrainState>,
	mut raw_roots: Query<(&PresentedTerrainScene, &mut Visibility)>,
) {
	for (presented, mut visibility) in &mut raw_roots {
		let replaced = state.wanted.contains(&presented.0);
		let desired = if replaced {
			Visibility::Hidden
		} else {
			Visibility::Inherited
		};
		if *visibility != desired {
			*visibility = desired;
		}
	}
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
}
