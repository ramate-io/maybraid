//! [`presentation<B, T>`]: marshals published `T` within `B` into Bevy hosts.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::StaticSystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use bevy::prelude::{
	App, Commands, CommandsSceneExt, Entity, IntoScheduleConfigs, Local, Plugin, Res, Transform,
	Update,
};

use crate::gen::{Id, Version};
use crate::hcsg::storage::StoredEntry;
use crate::lod_ref::LodRef;
use crate::scene::{lod_host_scene_pending, SemanticLodScene};

use super::bounds::HcsgBounds;
use super::context::GenerationScheme;
use super::demand::{HcsgDemand, SubscriptionId};
use super::node::HcsgNode;
use super::runtime::{ensure_runtime, HcsgSystems};
use super::storage::{Busy, HcsgStorage};

struct Host {
	entity: Entity,
	bounds: Aabb3d,
	version: Version,
}

/// Per-system state for [`presentation`]: its subscription, how far it has
/// read, and the hosts it spawned.
pub struct Presented<T> {
	subscription: Option<SubscriptionId>,
	requested: Option<Aabb3d>,
	cursor: usize,
	hosts: HashMap<Id, Host>,
	/// Outer bounds hosts were last retired against; `None` until first run.
	retired_against: Option<Option<Aabb3d>>,
	_t: PhantomData<fn() -> T>,
}

impl<T> Default for Presented<T> {
	fn default() -> Self {
		Self {
			subscription: None,
			requested: None,
			cursor: 0,
			hosts: HashMap::new(),
			retired_against: None,
			_t: PhantomData,
		}
	}
}

/// Each frame: replace the subscription if `B::inner` changed, spawn hosts
/// for newly published ids, and despawn hosts whose bounds left `B::outer`.
///
/// Reads never wait on a lock: a busy store or demand defers to next frame.
/// Retiring a host never evicts its stored value.
pub fn presentation<B, T>(
	bounds: StaticSystemParam<B::Param>,
	storage: Res<HcsgStorage>,
	demand: Res<HcsgDemand>,
	mut state: Local<Presented<T>>,
	mut commands: Commands,
) where
	B: HcsgBounds,
	T: GenerationScheme + SemanticLodScene,
{
	let focus = B::focus(&bounds);
	state.request(&demand, B::inner(&bounds), focus);
	state.spawn(&storage, &demand, focus, &mut commands);
	state.retire(B::outer(&bounds), &mut commands);
}

impl<T: GenerationScheme + SemanticLodScene> Presented<T> {
	fn request(&mut self, demand: &HcsgDemand, inner: Option<Aabb3d>, focus: Option<Vec3>) {
		if inner == self.requested && self.subscription.is_some() == inner.is_some() {
			return;
		}
		self.requested = inner;
		self.cursor = 0;
		self.subscription = match inner {
			Some(region) => Some(demand.subscribe::<T>(self.subscription, region, focus)),
			None => {
				if let Some(subscription) = self.subscription.take() {
					demand.unsubscribe(subscription);
				}
				None
			}
		};
	}

	fn spawn(
		&mut self,
		storage: &HcsgStorage,
		demand: &HcsgDemand,
		focus: Option<Vec3>,
		commands: &mut Commands,
	) {
		let Some(subscription) = self.subscription else {
			return;
		};
		let ids = match demand.try_read_published(subscription, self.cursor) {
			Ok(Some(ids)) => ids,
			Ok(None) => {
				self.subscription = None;
				return;
			}
			Err(Busy) => return,
		};
		for id in ids {
			let Ok(entry) = storage.try_entry::<T>(id) else {
				break;
			};
			self.cursor += 1;
			let Some(entry) = entry else {
				continue;
			};
			if self.hosts.get(&id).is_some_and(|host| host.version == entry.version) {
				continue;
			}
			if let Some(stale) = self.hosts.remove(&id) {
				commands.entity(stale.entity).try_despawn();
			}
			let (bounds, version) = (entry.bounds, entry.version);
			let entity = spawn_host(commands, id, entry, focus);
			self.hosts.insert(id, Host { entity, bounds, version });
			self.retired_against = None;
		}
	}

	fn retire(&mut self, outer: Option<Aabb3d>, commands: &mut Commands) {
		if self.retired_against == Some(outer) {
			return;
		}
		self.retired_against = Some(outer);
		self.hosts.retain(|_, host| {
			let keep = outer.is_some_and(|outer| outer.intersects(&host.bounds));
			if !keep {
				commands.entity(host.entity).try_despawn();
			}
			keep
		});
	}
}

fn spawn_host<T: GenerationScheme + SemanticLodScene>(
	commands: &mut Commands,
	id: Id,
	entry: StoredEntry<Arc<T>>,
	focus: Option<Vec3>,
) -> Entity {
	let viewer = focus.map_or(Transform::IDENTITY, Transform::from_translation);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &entry.bounds,
	};
	let level = entry.value.scene_lod_level(&lod_ref);
	let node = HcsgNode { id, version: entry.version, bounds: entry.bounds, value: entry.value };
	commands
		.spawn_scene(lod_host_scene_pending(level, node.bounds))
		.insert(node)
		.id()
}

/// Presents `T` within `B`: requests its generation and keeps one
/// [`HcsgNode<T>`] host per published value.
///
/// Scenes come from the LOD refresh plugins registered for `HcsgNode<T>`.
pub struct PresentationPlugin<B, T>(PhantomData<fn() -> (B, T)>);

impl<B, T> Default for PresentationPlugin<B, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B, T> Plugin for PresentationPlugin<B, T>
where
	B: HcsgBounds,
	T: GenerationScheme + SemanticLodScene,
{
	fn build(&self, app: &mut App) {
		ensure_runtime(app);
		app.add_systems(Update, presentation::<B, T>.in_set(HcsgSystems));
	}
}
