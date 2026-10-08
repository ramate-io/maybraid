//! [`HcsgDemand`]: what the worker should fill, one subscription per bounds source.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::Resource;

use crate::gen::{Id, OriginalId};

use super::context::{GenerationContext, GenerationScheme};
use super::storage::Busy;

/// Handle a generation or presentation system keeps for its one subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubscriptionId(u64);

pub(super) type Discover = fn(&mut GenerationContext, Aabb3d) -> Vec<Id>;
pub(super) type Generate = fn(&mut GenerationContext, Id) -> bool;

fn discover<T: GenerationScheme>(cx: &mut GenerationContext, region: Aabb3d) -> Vec<Id> {
	T::original_ids_for(cx, region).into_iter().map(|OriginalId(id)| id).collect()
}

fn generate<T: GenerationScheme>(cx: &mut GenerationContext, id: Id) -> bool {
	cx.get_or_generate::<T>(id).is_some()
}

struct Subscription {
	bounds: Aabb3d,
	focus: Option<Vec3>,
	discover: Discover,
	generate: Generate,
	/// Every discovered id whose value is available, in the order it landed.
	published: Vec<Id>,
	done: bool,
	/// Set when the subscription is replaced, removed, or its epoch ends.
	cancelled: Arc<AtomicBool>,
}

impl Subscription {
	fn cancel(&self) {
		self.cancelled.store(true, Ordering::Release);
	}
}

#[derive(Default)]
struct DemandState {
	subscriptions: HashMap<SubscriptionId, Subscription>,
	epoch: u64,
	working: bool,
	shutdown: bool,
}

#[derive(Default)]
struct DemandShared {
	next_id: AtomicU64,
	state: Mutex<DemandState>,
	/// Notified when there is work, or on shutdown.
	wake: Condvar,
	/// Notified when the worker finishes a subscription or runs out of work.
	idle: Condvar,
}

/// One subscription per generation or presentation system, filled newest
/// first by the [`super::HcsgWorker`].
///
/// A subscription's bounds never change: new bounds replace the subscription,
/// which cancels whatever the worker was still doing for the old one.
#[derive(Resource, Clone, Default)]
pub struct HcsgDemand(Arc<DemandShared>);

/// One subscription the worker is filling.
pub(super) struct Job {
	pub id: SubscriptionId,
	pub bounds: Aabb3d,
	pub focus: Option<Vec3>,
	pub discover: Discover,
	pub generate: Generate,
	pub cancelled: Arc<AtomicBool>,
}

impl HcsgDemand {
	fn lock(&self) -> MutexGuard<'_, DemandState> {
		self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
	}

	fn try_lock(&self) -> Result<MutexGuard<'_, DemandState>, Busy> {
		match self.0.state.try_lock() {
			Ok(state) => Ok(state),
			Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
			Err(TryLockError::WouldBlock) => Err(Busy),
		}
	}

	/// Replaces `previous` (if any) with a subscription to `T` over `bounds`,
	/// under one lock, and wakes the worker.
	pub fn subscribe<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		bounds: Aabb3d,
		focus: Option<Vec3>,
	) -> SubscriptionId {
		let id = SubscriptionId(self.0.next_id.fetch_add(1, Ordering::Relaxed));
		let mut state = self.lock();
		if let Some(previous) = previous.and_then(|previous| state.subscriptions.remove(&previous))
		{
			previous.cancel();
		}
		state.subscriptions.insert(
			id,
			Subscription {
				bounds,
				focus,
				discover: discover::<T>,
				generate: generate::<T>,
				published: Vec::new(),
				done: false,
				cancelled: Arc::new(AtomicBool::new(false)),
			},
		);
		self.0.wake.notify_all();
		id
	}

	pub fn unsubscribe(&self, id: SubscriptionId) {
		if let Some(subscription) = self.lock().subscriptions.remove(&id) {
			subscription.cancel();
		}
		self.0.idle.notify_all();
	}

	/// Ids published for `id` from `cursor` on.
	///
	/// `Ok(None)` when the subscription no longer exists (replaced, removed,
	/// or dropped by [`Self::advance_epoch`]): subscribe again.
	pub fn try_read_published(
		&self,
		id: SubscriptionId,
		cursor: usize,
	) -> Result<Option<Vec<Id>>, Busy> {
		let state = self.try_lock()?;
		Ok(state.subscriptions.get(&id).map(|subscription| {
			let published = &subscription.published;
			published[cursor.min(published.len())..].to_vec()
		}))
	}

	/// Whether `id` still exists; `false` once replaced, removed, or dropped by
	/// [`Self::advance_epoch`].
	pub fn try_is_live(&self, id: SubscriptionId) -> Result<bool, Busy> {
		Ok(self.try_lock()?.subscriptions.contains_key(&id))
	}

	/// Ends the session: cancels and drops every subscription. Values the
	/// worker is still generating are not published.
	pub fn advance_epoch(&self) -> u64 {
		let mut state = self.lock();
		for (_, subscription) in state.subscriptions.drain() {
			subscription.cancel();
		}
		state.epoch += 1;
		self.0.idle.notify_all();
		state.epoch
	}

	pub fn epoch(&self) -> u64 {
		self.lock().epoch
	}

	/// Blocks until no subscription has outstanding work, or `timeout`
	/// passes. `false` on timeout. Needs a running [`super::HcsgWorker`].
	pub fn wait_idle(&self, timeout: Duration) -> bool {
		let deadline = Instant::now() + timeout;
		let mut state = self.lock();
		loop {
			let busy = state.working || state.subscriptions.values().any(|s| !s.done);
			if !busy {
				return true;
			}
			let Some(left) = deadline.checked_duration_since(Instant::now()) else {
				return false;
			};
			state = self.0.idle.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
		}
	}

	/// The newest unfinished subscription, blocking until there is one.
	/// `None` once shut down.
	pub(super) fn next_job(&self) -> Option<Job> {
		let mut state = self.lock();
		loop {
			if state.shutdown {
				return None;
			}
			let newest = state
				.subscriptions
				.iter()
				.filter(|(_, subscription)| !subscription.done)
				.max_by_key(|(id, _)| **id)
				.map(|(id, subscription)| Job {
					id: *id,
					bounds: subscription.bounds,
					focus: subscription.focus,
					discover: subscription.discover,
					generate: subscription.generate,
					cancelled: Arc::clone(&subscription.cancelled),
				});
			if let Some(job) = newest {
				state.working = true;
				return Some(job);
			}
			self.0.idle.notify_all();
			state = self.0.wake.wait(state).unwrap_or_else(PoisonError::into_inner);
		}
	}

	pub(super) fn append(&self, id: SubscriptionId, published: Id) {
		if let Some(subscription) = self.lock().subscriptions.get_mut(&id) {
			subscription.published.push(published);
		}
	}

	pub(super) fn finish(&self, job: &Job) {
		let mut state = self.lock();
		state.working = false;
		if let Some(subscription) = state.subscriptions.get_mut(&job.id) {
			subscription.done = true;
		}
		self.0.idle.notify_all();
	}

	pub(super) fn shutdown(&self) {
		self.lock().shutdown = true;
		self.0.wake.notify_all();
	}
}
