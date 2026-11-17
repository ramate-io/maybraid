//! Subscription registry: subscribe, read published ids, and advance epochs.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, TryLockError};

use bevy::math::Vec3;
use bevy::shape::Aabb3d;

use crate::gen::Id;

use super::super::bounds::HcsgClass;
use super::super::context::GenerationScheme;
use super::super::storage::Busy;
use super::schedule::min_pass;
use super::state::{discover, generate, lock_mutex, Subscription, SubscriptionId};
use super::HcsgDemand;

/// Maximum publication IDs copied and consumed by one frame-side read.
pub const PUBLICATION_PAGE_IDS: usize = 32;

/// One read of a subscription's published ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
	/// Ids published from the read's cursor on.
	pub ids: Vec<Id>,
	/// No more ids will be published: `ids` reaches the end.
	pub done: bool,
	/// Discovery/build failed; a partial publication is not authoritative.
	pub failed: bool,
	/// Session in which these IDs were published.
	pub epoch: u64,
}

impl HcsgDemand {
	/// Blocking setup/offline subscription. Frame adapters use the transactional
	/// nonblocking request synchronization path instead.
	///
	/// Replaces `previous` (if any) with a subscription to `T` over `regions`,
	/// under one lock, and wakes the worker.
	///
	/// A replacement inherits `previous`'s stride `pass`, floored at the
	/// current minimum among remaining live work, and its reach. A new
	/// subscription starts at that minimum, with reach `{T}`.
	pub fn subscribe<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		regions: Vec<Aabb3d>,
		focus: Option<Vec3>,
		class: HcsgClass,
	) -> SubscriptionId {
		self.subscribe_with_origin::<T>(previous, regions, focus, class, "external")
	}

	pub(in crate::hcsg) fn subscribe_with_origin<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		regions: Vec<Aabb3d>,
		focus: Option<Vec3>,
		class: HcsgClass,
		_origin: &'static str,
	) -> SubscriptionId {
		let id = SubscriptionId::new(self.0.next_id.fetch_add(1, Ordering::Relaxed));
		let mut state = self.lock();
		#[cfg(feature = "generation-tracing")]
		let mut cancelled = None;
		let inherited = previous
			.and_then(|previous_id| {
				state
					.subscriptions
					.remove(&previous_id)
					.map(|subscription| (previous_id, subscription))
			})
			.map(|(_previous_id, previous)| {
				previous.cancel();
				#[cfg(feature = "generation-tracing")]
				if let Some(trace) = &previous.trace {
					cancelled = Some((_previous_id.trace_raw(), trace.epoch));
				}
				(previous.pass, Arc::clone(&previous.reach))
			});
		let floor = min_pass(&state);
		let (pass, reach) = match inherited {
			Some((pass, reach)) => (pass.max(floor), reach),
			None => (floor, Arc::new(std::sync::Mutex::new(HashSet::new()))),
		};
		lock_mutex(&reach).insert(TypeId::of::<T>());
		#[cfg(feature = "generation-tracing")]
		let trace =
			super::super::trace::JobTrace::queued(_origin, std::any::type_name::<T>(), state.epoch);
		#[cfg(feature = "generation-tracing")]
		let enqueue = trace.clone();
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
				discovered_len: None,
				cursor: 0,
				published: Vec::new(),
				done: false,
				failed: false,
				ticket: None,
				cancelled: Arc::new(AtomicBool::new(false)),
				reach,
				#[cfg(feature = "generation-tracing")]
				trace,
			},
		);
		state.sweep_needed = true;
		self.0.wake.notify_all();
		drop(state);
		#[cfg(feature = "generation-tracing")]
		{
			if let Some((previous, epoch)) = cancelled {
				lod_generation_diagnostics::cancel(previous, epoch, "replaced");
			}
			if let Some(trace) = enqueue {
				trace.emit_enqueue(id.trace_raw());
			}
		}
		id
	}

	#[cfg(test)]
	pub(crate) fn unsubscribe(&self, id: SubscriptionId) {
		let mut state = self.lock();
		#[cfg(feature = "generation-tracing")]
		let mut cancelled = None;
		if let Some(subscription) = state.subscriptions.remove(&id) {
			subscription.cancel();
			#[cfg(feature = "generation-tracing")]
			if let Some(trace) = &subscription.trace {
				cancelled = Some(trace.epoch);
			}
			state.sweep_needed = true;
			self.0.wake.notify_all();
		}
		self.0.idle.notify_all();
		drop(state);
		#[cfg(feature = "generation-tracing")]
		if let Some(epoch) = cancelled {
			lod_generation_diagnostics::cancel(id.trace_raw(), epoch, "unsubscribed");
		}
	}

	/// Align one frame request under a single nonblocking transaction. On Busy,
	/// neither the registry nor the caller's request identity has changed.
	pub(in crate::hcsg) fn try_sync_subscription<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		already_requested: bool,
		regions: &[Aabb3d],
		focus: Option<Vec3>,
		class: HcsgClass,
		_origin: &'static str,
	) -> Result<Option<(Option<SubscriptionId>, bool)>, Busy> {
		let mut state = self.try_lock()?;
		let live = previous.is_some_and(|id| state.subscriptions.contains_key(&id));
		let wants = !regions.is_empty();
		if already_requested && live == wants {
			return Ok(None);
		}
		let epoch_advanced = previous.is_some() && !live;
		// Replacements share their predecessor's reach with an in-flight worker.
		// Acquire it before cancellation/removal, and never block under demand.
		let reach = previous
			.and_then(|id| state.subscriptions.get(&id))
			.map(|subscription| Arc::clone(&subscription.reach))
			.unwrap_or_else(|| Arc::new(std::sync::Mutex::new(HashSet::new())));
		if wants {
			let mut reached = match reach.try_lock() {
				Ok(reached) => reached,
				Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
				Err(TryLockError::WouldBlock) => return Err(Busy),
			};
			reached.insert(TypeId::of::<T>());
		}
		#[cfg(feature = "generation-tracing")]
		let mut cancelled = None;
		let inherited_pass = previous.and_then(|id| state.subscriptions.remove(&id)).map(|old| {
			old.cancel();
			#[cfg(feature = "generation-tracing")]
			if let Some(trace) = &old.trace {
				cancelled = previous.map(|id| (id.trace_raw(), trace.epoch));
			}
			old.pass
		});
		let id = if wants {
			let floor = min_pass(&state);
			let pass = inherited_pass.map_or(floor, |pass| pass.max(floor));
			let id = SubscriptionId::new(self.0.next_id.fetch_add(1, Ordering::Relaxed));
			#[cfg(feature = "generation-tracing")]
			let trace = super::super::trace::JobTrace::queued(
				_origin,
				std::any::type_name::<T>(),
				state.epoch,
			);
			state.subscriptions.insert(
				id,
				Subscription {
					regions: regions.to_vec(),
					focus,
					class,
					pass,
					discover: discover::<T>,
					generate: generate::<T>,
					discovered: None,
					discovered_len: None,
					cursor: 0,
					published: Vec::new(),
					done: false,
					failed: false,
					ticket: None,
					cancelled: Arc::new(AtomicBool::new(false)),
					reach,
					#[cfg(feature = "generation-tracing")]
					trace,
				},
			);
			Some(id)
		} else {
			None
		};
		#[cfg(feature = "generation-tracing")]
		let enqueue = id.and_then(|id| {
			state.subscriptions.get(&id).and_then(|subscription| subscription.trace.clone())
		});
		state.sweep_needed = true;
		self.0.wake.notify_all();
		self.0.idle.notify_all();
		drop(state);
		#[cfg(feature = "generation-tracing")]
		{
			if let Some((old, epoch)) = cancelled {
				lod_generation_diagnostics::cancel(
					old,
					epoch,
					if wants { "replaced" } else { "unsubscribed" },
				);
			}
			if let (Some(id), Some(trace)) = (id, enqueue) {
				trace.emit_enqueue(id.trace_raw());
			}
		}
		Ok(Some((id, epoch_advanced)))
	}

	/// Ids published for `id` from `cursor` on.
	///
	/// `Ok(None)` when the subscription no longer exists (replaced, removed,
	/// or dropped by [`Self::advance_epoch`]): subscribe again.
	#[cfg(test)]
	pub(crate) fn try_read_published(
		&self,
		id: SubscriptionId,
		cursor: usize,
	) -> Result<Option<Vec<Id>>, Busy> {
		Ok(self.try_read_limit(id, cursor, usize::MAX)?.map(|published| published.ids))
	}

	/// Read at most [`PUBLICATION_PAGE_IDS`] IDs in publication order, without
	/// waiting on a lock. `done` certifies worker completion and that this page
	/// reaches its tail. Advance the cursor only after consuming these IDs.
	pub fn try_read(&self, id: SubscriptionId, cursor: usize) -> Result<Option<Published>, Busy> {
		self.try_read_limit(id, cursor, PUBLICATION_PAGE_IDS)
	}

	fn try_read_limit(
		&self,
		id: SubscriptionId,
		cursor: usize,
		limit: usize,
	) -> Result<Option<Published>, Busy> {
		let state = self.try_lock()?;
		Ok(state.subscriptions.get(&id).map(|subscription| {
			let published = &subscription.published;
			let start = cursor.min(published.len());
			let end = start.saturating_add(limit).min(published.len());
			Published {
				ids: published[start..end].to_vec(),
				done: subscription.done && end == published.len(),
				failed: subscription.failed,
				epoch: state.epoch,
			}
		}))
	}

	/// Revalidate a page after a nonblocking storage read, before exposing it.
	pub(in crate::hcsg) fn try_page_is_live(
		&self,
		id: SubscriptionId,
		epoch: u64,
	) -> Result<bool, Busy> {
		let state = self.try_lock()?;
		Ok(state.epoch == epoch && state.subscriptions.contains_key(&id))
	}

	/// Whether `id` still exists; `false` once replaced, removed, or dropped by
	/// [`Self::advance_epoch`].
	#[cfg(test)]
	pub(crate) fn try_is_live(&self, id: SubscriptionId) -> Result<bool, Busy> {
		Ok(self.try_lock()?.subscriptions.contains_key(&id))
	}

	/// Ends the session: cancels and drops every subscription. Values the
	/// worker is still generating are not published.
	pub(crate) fn advance_epoch(&self) -> u64 {
		let mut state = self.lock();
		#[cfg(feature = "generation-tracing")]
		let mut cancelled = Vec::new();
		for (_id, subscription) in state.subscriptions.drain() {
			subscription.cancel();
			#[cfg(feature = "generation-tracing")]
			if let Some(trace) = &subscription.trace {
				cancelled.push((_id.trace_raw(), trace.epoch));
			}
		}
		state.epoch += 1;
		state.sweep_needed = true;
		state.epoch_sweep_pending = true;
		self.0.wake.notify_all();
		self.0.idle.notify_all();
		let epoch = state.epoch;
		drop(state);
		#[cfg(feature = "generation-tracing")]
		for (id, epoch) in cancelled {
			lod_generation_diagnostics::cancel(id, epoch, "epoch");
		}
		epoch
	}
}

#[cfg(test)]
mod tests;
