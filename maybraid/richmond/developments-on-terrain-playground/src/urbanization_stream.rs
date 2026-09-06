//! Urbanization generate / host glue (forest_stream parallel).
//!
//! Registers LOD generate for [`SelectedUrbanization`] and a present-keep
//! bullseye. Host spawn / cull runs in Update so it can own
//! [`DevelopmentIndex`] without conflicting with the urbanization LOD drain.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::log::info_span;
use bevy::prelude::*;
use durham_terrain::shaders::DurhamTerrainShader;
use durham_terrain_models::PresentedTerrainScene;
use lod::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, OriginalId, SpatialIndex, Version,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion, RegionPresenter};
use lod::{LodGenerateBudget, LodPresentRegionPlugin, LodSceneRefreshRegionPlugin, LodViewer};
use lod_avian::AvianLodSceneRefreshPlugin;
use richmond_development_models::{
	BuiltDevelopment, DevelopmentCell, DevelopmentEntryStore, DevelopmentHosts, DevelopmentIndex,
	PaddedStoreView, PaddedTerrainPresenter, PresentedPaddedTerrainScene, TerrainWithPads,
};
use richmond_urbanization::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationGenerationPlugin, UrbanizationIndex, UrbanizationKind, UrbanizationLodChan,
	UrbanizationPresentBullseye, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

pub use richmond_urbanization::{
	parse_urbanization_kind, stream_radii_m, UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS,
};

use crate::hosts::DevelopmentHostRoot;

/// Generate + present-keep plugins for [`SelectedUrbanization`].
///
/// Generate comes from [`UrbanizationGenerationPlugin`]. This also registers
/// present-keep and padded-terrain scene refresh used by the playground.
pub fn register_urbanization_lod(app: &mut App) {
	#[derive(Debug, Clone, Copy, Default)]
	struct PaddedTerrainRefresh;

	if !app.is_plugin_added::<UrbanizationGenerationPlugin>() {
		app.add_plugins(UrbanizationGenerationPlugin);
	}
	app.init_resource::<UrbanizationPresenterState>()
		.init_resource::<UrbanizationPresentBullseye>()
		.init_resource::<UrbanizationHostBudget>()
		.init_resource::<UrbanizationPaddedTerrainBudget>()
		.insert_resource(LodGenerateBudget { ids_per_frame: 8 })
		.add_plugins(LodPresentRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default())
		.add_plugins(LodSceneRefreshRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			PaddedTerrainRefresh,
		>::default())
		.add_plugins(AvianLodSceneRefreshPlugin::<
			TerrainWithPads,
			PaddedTerrainRefresh,
			With<LodViewer>,
		>::default());
}

/// Cap host spawn to this many unpresented urbanization cells per frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct UrbanizationHostBudget {
	pub cells_per_frame: usize,
}

impl Default for UrbanizationHostBudget {
	fn default() -> Self {
		Self { cells_per_frame: 1 }
	}
}

/// Cap pad bake to this many [`TerrainWithPads`] ids per frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct UrbanizationPaddedTerrainBudget {
	pub ids_per_frame: usize,
}

impl Default for UrbanizationPaddedTerrainBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
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

	/// Generate leaf developments and spawn hosts for one urbanization cell.
	pub fn present_urbanization(
		&mut self,
		commands: &mut Commands,
		development: &mut DevelopmentIndex,
		id: Id,
		version: Version,
		selected: &SelectedUrbanization,
		lod_ref: &LodRef,
	) -> Vec<Entity> {
		if let Some(previous) = self.retire(id) {
			self.pending_despawn.push_back(previous.entities);
		}

		let mut entities = Vec::new();
		for leaf in &selected.leaves {
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			{
				let _span = info_span!("richmond_development_generation").entered();
				if GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
					development,
					leaf_id,
					lod_ref,
				)
				.is_none()
				{
					continue;
				}
				if GeneratingSpatialIndex::<BuiltDevelopment>::get_or_generate(
					development,
					leaf_id,
					lod_ref,
				)
				.is_none()
				{
					continue;
				}
			}
			let Some(built) = SpatialIndex::<BuiltDevelopment>::get(development, leaf_id) else {
				continue;
			};
			{
				let _span = info_span!("richmond_host_spawn").entered();
				entities.extend(spawn_tagged_hosts(commands, built));
			}
		}

		self.presented
			.insert(id, PresentedUrbanization { version, entities: entities.clone() });
		entities
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

/// Expand presented urbanization cells into BuiltDevelopment hosts and cull stale ones.
pub fn present_urbanization_hosts(
	mut commands: Commands,
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	budget: Res<UrbanizationHostBudget>,
	mut development: DevelopmentIndex,
	mut state: ResMut<UrbanizationPresenterState>,
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

	let tracked: Vec<(Id, Version)> =
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*development.urbanization, region)
			.into_iter()
			.filter_map(|tracked| {
				let id = tracked.0;
				let version =
					SpatialIndex::<SelectedUrbanization>::version(&*development.urbanization, id)?;
				Some((id, version))
			})
			.collect();

	let wanted: HashSet<Id> = tracked.iter().map(|(id, _)| *id).collect();
	let mut remaining = budget.cells_per_frame;
	for (id, version) in tracked {
		if state.presented_version(id) == Some(version) {
			continue;
		}
		if remaining == 0 {
			continue;
		}
		let Some(selected) = development.urbanization.get(id).cloned() else {
			continue;
		};
		state.present_urbanization(
			&mut commands,
			&mut development,
			id,
			version,
			&selected,
			&lod_ref,
		);
		remaining -= 1;
	}
	state.remove_stale(&mut commands, &wanted);
}

/// Bake a bounded number of padded Durham cells after hosts have written pads.
///
/// Skip ids whose Durham cell is not in the terrain store yet. Do not
/// `get_or_generate_region` the whole present keep.
pub fn generate_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	budget: Res<UrbanizationPaddedTerrainBudget>,
	mut development: DevelopmentIndex,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
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
	let mut ids: Vec<Id> = TerrainWithPads::original_ids_for(&mut development, region)
		.into_iter()
		.map(|OriginalId(id)| id)
		.collect();
	ids.sort();
	let mut remaining = budget.ids_per_frame;
	for id in ids {
		if remaining == 0 {
			break;
		}
		if SpatialIndex::<TerrainWithPads>::get(&development, id).is_some() {
			continue;
		}
		if development.terrain_store().terrain(id).is_none() {
			continue;
		}
		let _ = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate(
			&mut development,
			id,
			&lod_ref,
		);
		remaining -= 1;
	}
}

/// Present padded replacements for the urbanization keep and cull stale cells.
pub fn present_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	viewers: Query<&Transform, With<LodViewer>>,
) {
	state.wanted.clear();
	let Some(region) = keep.region.filter(|_| config.urbanization.is_some()) else {
		presenter.remove_stale(&state.wanted);
		return;
	};
	let viewer = viewers.single().copied().unwrap_or(Transform::IDENTITY);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let view = PaddedStoreView::new(&store);
	RegionPresenter::<TerrainWithPads, _>::present(&mut presenter, &view, region, &lod_ref);
	state.wanted = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	presenter.remove_stale(&state.wanted);
}

/// Hide raw Durham visuals while their padded replacements are active.
///
/// Raw collision stays live: stable terrain colliders are independent of visual
/// LOD and must never be disabled during a raw/padded presentation handoff.
pub fn sync_raw_terrain_replacements(
	mut commands: Commands,
	state: Res<UrbanizationPaddedTerrainState>,
	raw_roots: Query<(Entity, &PresentedTerrainScene)>,
	padded_roots: Query<(Entity, &PresentedPaddedTerrainScene)>,
	children: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
	raw_terrain_meshes: Query<(), With<MeshMaterial3d<DurhamTerrainShader>>>,
) {
	let ready: HashSet<Id> = padded_roots
		.iter()
		.filter(|(root, _)| children.iter_descendants(*root).any(|child| meshes.contains(child)))
		.map(|(_, presented)| presented.0)
		.collect();
	for (root, presented) in &raw_roots {
		// Keep raw collision live before, during, and after the visual handoff.
		let replaced = state.wanted.contains(&presented.0) && ready.contains(&presented.0);
		commands.entity(root).insert(Visibility::Inherited);
		for child in children.iter_descendants(root) {
			if raw_terrain_meshes.contains(child) {
				commands.entity(child).insert(if replaced {
					Visibility::Hidden
				} else {
					Visibility::Inherited
				});
			}
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

	#[test]
	fn host_and_pad_budgets_default_to_one_id_per_frame() {
		assert_eq!(UrbanizationHostBudget::default().cells_per_frame, 1);
		assert_eq!(UrbanizationPaddedTerrainBudget::default().ids_per_frame, 1);
	}
}
