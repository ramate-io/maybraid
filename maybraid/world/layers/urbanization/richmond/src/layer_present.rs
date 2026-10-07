//! Host spawn and padded-cell presentation for [`crate::Richmond`].

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::Water;
use durham::{
	PresentedTerrainScene, TerrainColliderMeshSource, TerrainSuperseded, TerrainTrimeshCollider,
};
use layer_stack::LodPresentGateSync;
use lod::gen::{Id, SpatialIndex, Version};
use lod::hcsg::{HcsgStorage, LodGenerateSystems};
use lod::lod_ref::LodRef;
use lod::{LodPresentGate, LodPresentSystems, LodViewer};
use terrain_layer_model::{terrain_streaming, TerrainExtent, TerrainLayerSystems};
use urbanization_layer_model::{
	urbanization_host_region, urbanization_visual_region, UrbanSetting, Urbanization,
	UrbanizationGenerationSystems, UrbanizationLayerRegion,
};

use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::host::DevelopmentHosts;
use crate::layer::Richmond;
use crate::padded::PaddedTerrain;
use crate::presentation::PaddedTerrainPresenter;
use crate::storage::RichmondStorage;
use crate::{BuiltDevelopment, PresentedPaddedTerrainScene};

#[derive(Component)]
pub struct DevelopmentHostRoot;

pub fn spawn_tagged_host_entities(
	commands: &mut Commands,
	development: &impl DevelopmentHosts,
	host_id: Option<Id>,
) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		for entity in host.spawn(commands, host_id) {
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
	spawn_tagged_host_entities(commands, development, None).len()
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
				commands.entity(*entity).try_despawn();
			}
		}
		self.presented.clear();
		for entities in self.pending_despawn.drain(..) {
			for entity in entities {
				commands.entity(entity).try_despawn();
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
				commands.entity(entity).try_despawn();
			}
		}
	}

	/// Spawns `built`'s hosts and an [`UrbanSetting`] at `elevation` (the
	/// leaf's first pad), replacing an older version of the leaf.
	pub fn present_leaf(
		&mut self,
		commands: &mut Commands,
		leaf_id: Id,
		version: Version,
		elevation: Option<f32>,
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
		let elevation = elevation.unwrap_or(center.y);
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
		entities.extend(spawn_tagged_host_entities(commands, built, Some(leaf_id)));
		self.presented.insert(leaf_id, PresentedUrbanization { version, entities });
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UrbanizationHostPresent;

pub fn present_richmond_hosts<G>(
	mut commands: Commands,
	gate: Res<LodPresentGate<Urbanization<Richmond<G>>>>,
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	storage: Res<HcsgStorage>,
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
	for (id, version, built) in storage.built_overlapping::<G>(region) {
		let Some(development) = storage.get::<RichmondDevelopment<G>>(id) else {
			continue;
		};
		let elevation = development.pads().first().map(|pad| pad.height);
		state.present_leaf(&mut commands, id, version, elevation, built, development.cell());
		wanted.insert(id);
	}
	state.remove_stale(&mut commands, &wanted);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaddedTerrainTickKey {
	region: Aabb3d,
	padded_rev: u64,
	water_rev: u64,
	viewer: (i32, i32),
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
	gate: Res<LodPresentGate<Urbanization<Richmond<G>>>>,
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	storage: Res<HcsgStorage>,
	mut presenter: PaddedTerrainPresenter,
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
		padded_rev: SpatialIndex::<PaddedTerrain<G>>::membership_revision(&*storage),
		water_rev: SpatialIndex::<Water>::membership_revision(&*storage),
		viewer: quantize_viewer_xz(viewer.translation),
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
	let tracked: HashSet<Id> =
		storage.overlapping::<PaddedTerrain<G>>(region).into_iter().collect();
	presenter.present_tracked::<G>(&storage, &tracked, &lod_ref);
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
					commands.entity(entity).try_insert(TerrainSuperseded);
				}
				now_replaced.insert(presented.0);
			}
		} else if state.replaced.contains(&presented.0) {
			*visibility = Visibility::Inherited;
			if superseded {
				commands.entity(entity).try_remove::<TerrainSuperseded>();
			}
		}
	}
	state.replaced = now_replaced;
}

pub(crate) fn install_richmond_presentation<G: RichmondGround>(app: &mut App)
where
	Urbanization<Richmond<G>>: terrain_layer_model::TerrainModel,
{
	app.init_resource::<UrbanizationPresenterState>()
		.init_resource::<LodPresentGate<Urbanization<Richmond<G>>>>();
	app.add_systems(
		Update,
		present_richmond_hosts::<G>
			.in_set(UrbanizationHostPresent)
			.after(UrbanizationGenerationSystems)
			.after(LodGenerateSystems::Drain)
			.after(LodPresentGateSync)
			.run_if(terrain_streaming::<G>)
			.before(LodPresentSystems::Produce)
			.before(TerrainLayerSystems::<G::Base>::QueueColliders),
	);
	app.init_resource::<UrbanizationPaddedTerrainState>().add_systems(
		Update,
		(present_richmond_padded_terrain::<G>, sync_raw_terrain_replacements)
			.chain()
			.after(UrbanizationGenerationSystems)
			.after(LodGenerateSystems::Drain)
			.after(UrbanizationHostPresent)
			.after(LodPresentGateSync)
			.run_if(terrain_streaming::<G>)
			.before(LodPresentSystems::Produce)
			.before(TerrainLayerSystems::<G::Base>::QueueColliders),
	);
}
