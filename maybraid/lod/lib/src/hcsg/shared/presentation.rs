//! [`presentation<B, T>`]: marshals published `T` within `B` into Bevy hosts.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::system::StaticSystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use bevy::prelude::{
	Allow, App, Commands, CommandsSceneExt, Component, Entity, IntoScheduleConfigs, Local, Plugin,
	Query, Res, Transform, Update, With,
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
	/// The session that last published this host's value.
	session: u64,
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
	/// Advances when an epoch drops the subscription.
	session: u64,
	/// Hosts may remain from an earlier session.
	sweep_pending: bool,
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
			session: 0,
			sweep_pending: false,
			_t: PhantomData,
		}
	}
}

/// Each frame: replace the subscription if `B::inner` changed, spawn hosts
/// for newly published ids, and retire hosts whose bounds left `B::outer`.
/// Retired hosts despawn in `Last` ([`RetiredHost`]).
///
/// Across an epoch, hosts keep showing the previous session until its values
/// are replaced by version. Once the new session's subscription is done,
/// hosts it never published are retired.
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
		let published = match demand.try_read(subscription, self.cursor) {
			Ok(Some(published)) => published,
			Ok(None) => {
				self.subscription = None;
				self.session += 1;
				self.sweep_pending = !self.hosts.is_empty();
				return;
			}
			Err(Busy) => return,
		};
		let mut read_all = true;
		for id in published.ids {
			let Ok(entry) = storage.try_entry::<T>(id) else {
				read_all = false;
				break;
			};
			self.cursor += 1;
			let Some(entry) = entry else {
				continue;
			};
			if let Some(host) = self.hosts.get_mut(&id).filter(|host| host.version == entry.version)
			{
				host.session = self.session;
				continue;
			}
			if let Some(stale) = self.hosts.remove(&id) {
				retire_host(commands, stale.entity);
			}
			let (bounds, version) = (entry.bounds, entry.version);
			let entity = spawn_host(commands, id, entry, focus);
			self.hosts.insert(id, Host { entity, bounds, version, session: self.session });
			self.retired_against = None;
		}
		if read_all && published.done && self.sweep_pending {
			self.sweep_pending = false;
			let session = self.session;
			self.hosts.retain(|_, host| {
				let current = host.session == session;
				if !current {
					retire_host(commands, host.entity);
				}
				current
			});
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
				retire_host(commands, host.entity);
			}
			keep
		});
	}
}

/// A host presentation has let go of. It despawns in `Last`, so commands other
/// systems queue on it, or on what it spawned, through `PostUpdate` still land.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RetiredHost;

fn retire_host(commands: &mut Commands, entity: Entity) {
	commands.entity(entity).try_insert(RetiredHost);
}

pub(super) fn despawn_retired_hosts(
	retired: Query<Entity, (With<RetiredHost>, Allow<Disabled>)>,
	mut commands: Commands,
) {
	for entity in &retired {
		commands.entity(entity).try_despawn();
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
