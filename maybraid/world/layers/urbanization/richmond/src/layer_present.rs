//! Host spawn and padded-cell presentation for [`crate::Richmond`].

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::{
	PresentedTerrainScene, TerrainColliderMeshSource, TerrainSuperseded, TerrainTrimeshCollider,
};
use furniture_assemblies::{
	FurnitureAssembliesPlugin, FurnitureStreamPlugin, FurnitureStreamSystems,
};
use furniture_shaders::FurnitureShadersPlugin;
use layer_stack::LodPresentGateSync;
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::{LodPresentGate, LodPresentSystems, LodViewer};
use terrain_layer_model::{terrain_streaming, TerrainExtent, TerrainLayerSystems};
use urbanization_layer_model::UrbanizationStoreSystems;
use urbanization_layer_model::{
	urbanization_host_region, urbanization_visual_region, UrbanSetting, Urbanization,
	UrbanizationGenerationSystems, UrbanizationLayerRegion,
};
use urbanization_layer_presentation::{PaddedCells, UrbanizationHosts, UrbanizationPresentation};

use crate::development::DevelopmentCell;
use crate::ground::RichmondGround;
use crate::host::DevelopmentHosts;
use crate::index::{DevelopmentEntryStore, PaddedStoreView};
use crate::layer::Richmond;
use crate::padded::TerrainWithPads;
use crate::presentation::PaddedTerrainPresenter;
use crate::{BuiltDevelopment, PresentedPaddedTerrainScene};

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_tagged_host_entities(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		for entity in host.spawn(commands) {
			commands.entity(entity).insert(DevelopmentHostRoot);
			spawned.push(entity);
		}
	}
	spawned
}

pub fn spawn_development_hosts(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
) -> usize {
	spawn_tagged_host_entities(commands, development).len()
}

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

	#[cfg(test)]
	pub(crate) fn insert_presented_for_test(&mut self, id: Id, entities: Vec<Entity>) {
		self.presented
			.insert(id, PresentedUrbanization { version: Version(1), entities });
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

	pub fn present_leaf(
		&mut self,
		commands: &mut Commands,
		leaf_id: Id,
		version: Version,
		cell: &DevelopmentCell,
		built: &BuiltDevelopment,
		leaf_bounds: Aabb3d,
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
		entities.extend(spawn_tagged_host_entities(commands, built));
		self.presented.insert(leaf_id, PresentedUrbanization { version, entities });
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UrbanizationHostPresent;

pub fn present_richmond_hosts<G>(
	mut commands: Commands,
	gate: Res<LodPresentGate<(Urbanization<Richmond<G>>, UrbanizationHosts)>>,
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	store: Res<DevelopmentEntryStore>,
	mut state: ResMut<UrbanizationPresenterState>,
) where
	G: RichmondGround,
{
	if gate.is_changed() && !gate.open {
		state.clear(&mut commands);
		return;
	}
	if !gate.open {
		return;
	}
	let Some(region) = urbanization_host_region(&*extent, layer.region) else {
		state.clear(&mut commands);
		return;
	};

	let mut wanted = HashSet::new();
	for (id, version, built) in store.developments_overlapping_tracked(region) {
		let Some(cell) = store.cell(id) else {
			continue;
		};
		if !cell.is_filled() {
			continue;
		}
		state.present_leaf(&mut commands, id, version, cell, built, cell.cell);
		wanted.insert(id);
	}
	state.remove_stale(&mut commands, &wanted);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaddedTerrainTickKey {
	region: Aabb3d,
	store_rev: u64,
	terrain_rev: u64,
	viewer: Option<(i32, i32)>,
}

const PADDED_VIEWER_QUANT_XZ: f32 = 8.0;

fn quantize_viewer_xz(translation: Vec3) -> (i32, i32) {
	(
		(translation.x / PADDED_VIEWER_QUANT_XZ).floor() as i32,
		(translation.z / PADDED_VIEWER_QUANT_XZ).floor() as i32,
	)
}

#[cfg(test)]
pub(crate) fn quantize_viewer_xz_for_test(translation: Vec3) -> (i32, i32) {
	quantize_viewer_xz(translation)
}

#[derive(Resource, Default)]
pub struct UrbanizationPaddedTerrainState {
	pub(crate) wanted: HashSet<Id>,
	pub(crate) replaced: HashSet<Id>,
}

pub(crate) fn present_richmond_padded_terrain<G>(
	gate: Res<LodPresentGate<(Urbanization<Richmond<G>>, PaddedCells)>>,
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter<G>,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) where
	G: RichmondGround,
{
	let Some(region) = urbanization_visual_region(&*extent, layer.region).filter(|_| gate.open)
	else {
		if last.is_some() || !state.wanted.is_empty() {
			state.wanted.clear();
			presenter.remove_stale(&state.wanted);
		}
		*last = None;
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
	let key = PaddedTerrainTickKey {
		region,
		store_rev: store.membership_revision(),
		terrain_rev: presenter.terrain_membership_revision(),
		viewer: Some(quantize_viewer_xz(viewer.translation)),
	};
	if last.as_ref() == Some(&key) {
		return;
	}
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let view = PaddedStoreView::new(&store);
	let tracked: HashSet<Id> = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	presenter.present_tracked(&view, &tracked, &lod_ref);
	state.wanted = tracked;
	*last = Some(key);
}

pub fn sync_raw_terrain_replacements(
	mut commands: Commands,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	padded: Query<(
		&PresentedPaddedTerrainScene,
		Has<TerrainTrimeshCollider>,
		Has<TerrainColliderMeshSource>,
	)>,
	mut raw_roots: Query<(Entity, &PresentedTerrainScene, &mut Visibility, Has<TerrainSuperseded>)>,
) {
	let ready: HashSet<Id> = padded
		.iter()
		.filter(|(scene, cooked, collides)| {
			state.wanted.contains(&scene.0) && (*cooked || !*collides)
		})
		.map(|(scene, _, _)| scene.0)
		.collect();
	let mut now_replaced = HashSet::new();
	for (entity, presented, mut visibility, superseded) in &mut raw_roots {
		if ready.contains(&presented.0) {
			let ours = state.replaced.contains(&presented.0);
			if ours || *visibility != Visibility::Hidden {
				if *visibility != Visibility::Hidden {
					*visibility = Visibility::Hidden;
				}
				if !superseded {
					commands.entity(entity).insert(TerrainSuperseded);
				}
				now_replaced.insert(presented.0);
			}
		} else if state.replaced.contains(&presented.0) {
			*visibility = Visibility::Inherited;
			if superseded {
				commands.entity(entity).remove::<TerrainSuperseded>();
			}
		}
	}
	state.replaced = now_replaced;
}

impl<G: RichmondGround> UrbanizationPresentation for Richmond<G>
where
	Urbanization<Richmond<G>>: terrain_layer_model::TerrainModel,
{
	fn install_hosts(app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<FurnitureAssembliesPlugin>() {
			app.add_plugins(FurnitureAssembliesPlugin);
		}
		if !app.is_plugin_added::<FurnitureStreamPlugin<Urbanization<Richmond<G>>>>() {
			app.add_plugins(FurnitureStreamPlugin::<Urbanization<Richmond<G>>>::default());
		}
		app.configure_sets(
			Update,
			FurnitureStreamSystems::Generate.after(UrbanizationStoreSystems),
		);
		app.init_resource::<UrbanizationPresenterState>()
			.init_resource::<LodPresentGate<(Urbanization<Richmond<G>>, UrbanizationHosts)>>();
		app.add_systems(
			Update,
			present_richmond_hosts::<G>
				.in_set(UrbanizationHostPresent)
				.after(UrbanizationGenerationSystems)
				.after(LodPresentGateSync)
				.run_if(terrain_streaming::<G>)
				.before(LodPresentSystems::Produce)
				.before(TerrainLayerSystems::<G::Base>::QueueColliders),
		);
	}

	fn install_padded_cells(app: &mut App) {
		app.init_resource::<UrbanizationPaddedTerrainState>()
			.init_resource::<LodPresentGate<(Urbanization<Richmond<G>>, PaddedCells)>>()
			.add_systems(
				Update,
				(present_richmond_padded_terrain::<G>, sync_raw_terrain_replacements)
					.chain()
					.after(UrbanizationGenerationSystems)
					.after(UrbanizationHostPresent)
					.after(LodPresentGateSync)
					.run_if(terrain_streaming::<G>)
					.before(LodPresentSystems::Produce)
					.before(TerrainLayerSystems::<G::Base>::QueueColliders),
			);
	}
}
