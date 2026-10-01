//! [`UrbanizationPresentationPlugin`]: building hosts, building LOD, walk colliders.
//!
//! [`PaddedCells`] presents padded replacements for `Urbanization<M>` through
//! [`TerrainPresentationPlugin`](terrain_layer_presentation::TerrainPresentationPlugin).

use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use durham_terrain_models::{terrain_streaming_enabled, TerrainColliderSystems};
use furniture_assemblies::{
	FurnitureAssembliesPlugin, FurnitureStreamPlugin, FurnitureStreamSystems,
};
use furniture_shaders::FurnitureShadersPlugin;
use lod::gen::{Id, Version};
use lod::presentation::LodPresentKeepRegion;
use lod::LodPresentSystems;
use richmond_building_physics::BuildingWalkColliderPlugin;
use richmond_urbanization::{UrbanDevelopmentKind, UrbanizationLodChan};
use terrain_layer_model::{
	subscribe_mode, GenerationMode, ModeSubscription, TerrainView,
};
use urbanization_layer_model::{
	generate_urbanization_developments, UrbanModel, UrbanSetting, UrbanizationGenerationSystems,
	UrbanizationLayerConfig,
};

mod hosts;
mod padded;

pub use hosts::{spawn_development_hosts, spawn_tagged_host_entities, DevelopmentHostRoot};
pub use padded::{
	present_urbanization_padded_terrain, sync_raw_terrain_replacements, PaddedCells,
	UrbanizationPaddedTerrainState,
};

#[derive(Resource, Default)]
pub struct UrbanizationPresenterState {
	presented: HashMap<Id, PresentedUrbanization>,
	pending_despawn: VecDeque<Vec<Entity>>,
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

	/// Spawn hosts for one filled leaf that already has a built development.
	pub fn present_leaf(
		&mut self,
		commands: &mut Commands,
		leaf_id: Id,
		version: Version,
		cell: &richmond_development_models::DevelopmentCell,
		built: &richmond_development_models::BuiltDevelopment,
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
		entities.extend(hosts::spawn_tagged_host_entities(commands, built));
		self.presented.insert(leaf_id, PresentedUrbanization { version, entities });
	}
}

/// Marker for host-presenter subscriptions on ground `G`.
pub struct UrbanizationHosts;

/// GET-only host spawn for leaves that already have a built development.
///
/// Host teardown when the stream is off used to live in `stream_urbanization`
/// (`UrbanizationStreamLod` held presenter state). Clearing here keeps that
/// teardown without putting despawn in the model crate. The system
/// still sits after [`UrbanizationGenerationSystems`] and before padded present.
pub fn present_urbanization_hosts<G: UrbanModel>(
	mut commands: Commands,
	config: Res<UrbanizationLayerConfig>,
	subscription: ModeSubscription<(G, UrbanizationHosts)>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	view: TerrainView<G>,
	mut state: ResMut<UrbanizationPresenterState>,
) {
	let spec = config.urbanization.as_ref().filter(|_| subscription.active());
	if spec.is_none() {
		state.clear(&mut commands);
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	let urbanization_ids = G::urbanization_cell_ids(&view.read, region);
	let mut wanted = HashSet::new();
	for id in urbanization_ids {
		let Some(selected) = G::urbanization_cell(&view.read, id) else {
			continue;
		};
		for leaf in &selected.leaves {
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			let Some(cell) = G::development_cell(&view.read, leaf_id) else {
				continue;
			};
			let Some((built, version)) = G::built_at(&view.read, leaf_id) else {
				continue;
			};
			state.present_leaf(&mut commands, leaf_id, version, cell, built, leaf.bounds);
			wanted.insert(leaf_id);
		}
	}
	state.remove_stale(&mut commands, &wanted);
}

/// Host present, so padded present / raw sync can order after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UrbanizationHostPresent;

/// Presents the built developments of urbanized model `G` while `Mode` is subscribed.
pub struct UrbanizationPresentationPlugin<Mode, G>(PhantomData<fn() -> (Mode, G)>);

impl<Mode, G> Default for UrbanizationPresentationPlugin<Mode, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct UrbanizationPresentationCore<G>(PhantomData<fn() -> G>);

impl<G> Default for UrbanizationPresentationCore<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: UrbanModel> Plugin for UrbanizationPresentationCore<G> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<FurnitureAssembliesPlugin>() {
			app.add_plugins(FurnitureAssembliesPlugin);
		}
		if !app.is_plugin_added::<FurnitureStreamPlugin>() {
			app.add_plugins(FurnitureStreamPlugin);
		}
		if !app.is_plugin_added::<BuildingWalkColliderPlugin>() {
			app.add_plugins(BuildingWalkColliderPlugin);
		}
		app.init_resource::<UrbanizationPresenterState>();
		#[allow(private_interfaces)]
		app.configure_sets(
			Update,
			FurnitureStreamSystems::Generate.after(generate_urbanization_developments),
		);
		app.add_systems(
			Update,
			present_urbanization_hosts::<G>
				.in_set(UrbanizationHostPresent)
				.after(UrbanizationGenerationSystems)
				.run_if(terrain_streaming_enabled)
				.before(LodPresentSystems::Produce)
				.before(TerrainColliderSystems::QueueMeshes),
		);
	}
}

impl<Mode: GenerationMode, G: UrbanModel> Plugin for UrbanizationPresentationPlugin<Mode, G> {
	fn build(&self, app: &mut App) {
		subscribe_mode::<(G, UrbanizationHosts), Mode>(app);
		if !app.is_plugin_added::<UrbanizationPresentationCore<G>>() {
			app.add_plugins(UrbanizationPresentationCore::<G>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
