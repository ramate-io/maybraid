//! [`HcsgDemand`]: what the worker should fill, one subscription per system.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::Resource;

use crate::gen::{Id, OriginalId};

use super::bounds::HcsgClass;
use super::context::{GenerationContext, GenerationScheme};
use super::storage::Busy;

/// Ids the worker generates in one quantum before returning to the scheduler.
pub const QUANTUM_IDS: usize = 32;
/// Wall time after which a quantum yields, even if fewer ids have run.
pub const QUANTUM_TIME: Duration = Duration::from_millis(30);

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
	regions: Vec<Aabb3d>,
	focus: Option<Vec3>,
	class: HcsgClass,
	pass: f64,
	discover: Discover,
	generate: Generate,
	/// Deduped, focus-sorted ids, once discovery has run.
	discovered: Option<Vec<Id>>,
	/// Next index in [`Self::discovered`] to generate.
	cursor: usize,
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

struct DemandShared {
	next_id: AtomicU64,
	picks: AtomicU64,
	state: Mutex<DemandState>,
	/// Notified when there is work, or on shutdown.
	wake: Condvar,
	/// Notified when the worker finishes a quantum or runs out of work.
	idle: Condvar,
}

impl Default for DemandShared {
	fn default() -> Self {
		Self {
			next_id: AtomicU64::new(0),
			picks: AtomicU64::new(0),
			state: Mutex::new(DemandState::default()),
			wake: Condvar::new(),
			idle: Condvar::new(),
		}
	}
}

/// One subscription per generation or presentation system, filled in
/// weighted quanta by the [`super::HcsgWorker`].
///
/// A subscription's regions never change: new regions replace the subscription,
/// which cancels whatever the worker was still doing for the old one. A
/// replacement inherits the predecessor's stride `pass`, floored at the current
/// minimum among remaining live work, so resubscribing cannot jump the queue
/// or bank credit from a finished subscription.
#[derive(Resource, Clone, Default)]
pub struct HcsgDemand(Arc<DemandShared>);

/// One read of a subscription's published ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
	/// Ids published from the read's cursor on.
	pub ids: Vec<Id>,
	/// No more ids will be published: `ids` reaches the end.
	pub done: bool,
}

/// One subscription the worker is filling for a quantum.
pub(crate) struct Job {
	pub id: SubscriptionId,
	pub regions: Vec<Aabb3d>,
	pub focus: Option<Vec3>,
	pub discover: Discover,
	pub generate: Generate,
	pub cancelled: Arc<AtomicBool>,
	pub discovered: Option<Vec<Id>>,
	pub cursor: usize,
}

/// What the worker writes back after one quantum.
pub(crate) struct QuantumProgress {
	pub discovered: Option<Vec<Id>>,
	pub cursor: usize,
	pub cost: f64,
	pub done: bool,
}

/// Charge the larger of the id count and the time fraction of a quantum, so
/// an expensive class that hits the 30 ms cap after a few ids pays a full
/// quantum, not a handful of ids.
pub(crate) fn quantum_cost(ids: u32, elapsed: Duration) -> f64 {
	let by_ids = f64::from(ids);
	let by_time = (elapsed.as_secs_f64() / QUANTUM_TIME.as_secs_f64()) * QUANTUM_IDS as f64;
	by_ids.max(by_time)
}

fn min_pass(state: &DemandState) -> f64 {
	state
		.subscriptions
		.values()
		.filter(|subscription| !subscription.done)
		.map(|subscription| subscription.pass)
		.min_by(|a, b| a.total_cmp(b))
		.unwrap_or(0.0)
}

fn pick_job(state: &mut DemandState) -> Option<Job> {
	let id = state
		.subscriptions
		.iter()
		.filter(|(_, subscription)| !subscription.done)
		.min_by(|(id_a, a), (id_b, b)| a.pass.total_cmp(&b.pass).then(id_b.cmp(id_a)))
		.map(|(id, _)| *id)?;
	let subscription = state.subscriptions.get_mut(&id)?;
	Some(Job {
		id,
		regions: subscription.regions.clone(),
		focus: subscription.focus,
		discover: subscription.discover,
		generate: subscription.generate,
		cancelled: Arc::clone(&subscription.cancelled),
		discovered: subscription.discovered.take(),
		cursor: subscription.cursor,
	})
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

	/// Replaces `previous` (if any) with a subscription to `T` over `regions`,
	/// under one lock, and wakes the worker.
	///
	/// A replacement inherits `previous`'s stride `pass`, but never below the
	/// current minimum among remaining live work, so a finished subscription
	/// cannot bank credit. A new subscription starts at that minimum.
	pub fn subscribe<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		regions: Vec<Aabb3d>,
		focus: Option<Vec3>,
		class: HcsgClass,
	) -> SubscriptionId {
		let id = SubscriptionId(self.0.next_id.fetch_add(1, Ordering::Relaxed));
		let mut state = self.lock();
		let inherited =
			previous
				.and_then(|previous| state.subscriptions.remove(&previous))
				.map(|previous| {
					previous.cancel();
					previous.pass
				});
		let floor = min_pass(&state);
		let pass = inherited.map_or(floor, |pass| pass.max(floor));
		state.subscriptions.insert(
			id,
			Subscription {
				regions,
				focus,
				class,
				pass,
				discover: discover::<T>,
				generate: generate::<T>,
				discovered: None,
				cursor: 0,
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
		Ok(self.try_read(id, cursor)?.map(|published| published.ids))
	}

	/// Like [`Self::try_read_published`], read together with whether the
	/// worker has finished the subscription.
	pub fn try_read(&self, id: SubscriptionId, cursor: usize) -> Result<Option<Published>, Busy> {
		let state = self.try_lock()?;
		Ok(state.subscriptions.get(&id).map(|subscription| {
			let published = &subscription.published;
			Published {
				ids: published[cursor.min(published.len())..].to_vec(),
				done: subscription.done,
			}
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

	/// The unfinished subscription with the lowest stride `pass`, newest
	/// first among ties. Blocks until there is one. `None` once shut down.
	pub(super) fn next_job(&self) -> Option<Job> {
		let mut state = self.lock();
		loop {
			if state.shutdown {
				return None;
			}
			if let Some(job) = pick_job(&mut state) {
				state.working = true;
				self.0.picks.fetch_add(1, Ordering::Relaxed);
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

	pub(super) fn finish_quantum(&self, id: SubscriptionId, progress: QuantumProgress) {
		let mut state = self.lock();
		state.working = false;
		if let Some(subscription) = state.subscriptions.get_mut(&id) {
			subscription.discovered = progress.discovered;
			subscription.cursor = progress.cursor;
			if progress.done {
				subscription.done = true;
			} else if progress.cost > 0.0 {
				subscription.pass += progress.cost / f64::from(subscription.class.weight());
			}
		}
		self.0.idle.notify_all();
		self.0.wake.notify_all();
	}

	pub(super) fn shutdown(&self) {
		self.lock().shutdown = true;
		self.0.wake.notify_all();
	}

	/// How many times the worker has taken a quantum. Tests use this to check
	/// that a lone subscription yields once per quantum, not once per id.
	pub fn scheduler_picks(&self) -> u64 {
		self.0.picks.load(Ordering::Relaxed)
	}

	/// Non-blocking pick for scheduler tests. The caller must
	/// [`Self::finish_quantum`].
	#[cfg(test)]
	pub(crate) fn try_pick(&self) -> Option<Job> {
		let mut state = self.lock();
		let job = pick_job(&mut state)?;
		state.working = true;
		Some(job)
	}

	#[cfg(test)]
	pub(crate) fn pass_of(&self, id: SubscriptionId) -> Option<f64> {
		self.lock().subscriptions.get(&id).map(|subscription| subscription.pass)
	}
}
