//! Subscription registry: subscribe, read published ids, and advance epochs.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::math::Vec3;
use bevy::shape::Aabb3d;

use crate::gen::Id;

use super::super::bounds::HcsgClass;
use super::super::context::GenerationScheme;
use super::super::storage::Busy;
use super::schedule::min_pass;
use super::state::{discover, generate, lock_mutex, Subscription, SubscriptionId};
use super::HcsgDemand;

/// One read of a subscription's published ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
	/// Ids published from the read's cursor on.
	pub ids: Vec<Id>,
	/// No more ids will be published: `ids` reaches the end.
	pub done: bool,
}

impl HcsgDemand {
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
