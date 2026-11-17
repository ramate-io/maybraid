//! Bounded, shared gameplay requests over the existing demand worker.

use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::shape::Aabb3d;

use crate::hcsg::demand::schedule::min_pass;
use crate::hcsg::demand::state::{discover, generate, lock_mutex, DemandState, Subscription};
use crate::hcsg::demand::{HcsgDemand, SubscriptionId};
use crate::hcsg::{Busy, GenerationScheme, HcsgClass, HcsgStorage};

/// Maximum distinct live gameplay requests. Duplicate consumers share a slot.
pub const MAX_TICKET_REQUESTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketRequestError {
	Busy,
	Capacity,
	InvalidRegion,
}

impl From<Busy> for TicketRequestError {
	fn from(_: Busy) -> Self {
		Self::Busy
	}
}

pub(super) struct TicketLease {
	demand: HcsgDemand,
	cancelled: Arc<AtomicBool>,
}

impl Drop for TicketLease {
	fn drop(&mut self) {
		self.cancelled.store(true, Ordering::Release);
		self.demand.0.wake.notify_all();
	}
}

/// One consumer's cursor and completed-value snapshot. Last consumer release
/// cancels the shared subscription without acquiring a mutex. No generation
/// runs in request, poll, or drop. Dropping a ticket releases its retention.
pub struct HcsgTicket<T> {
	lease: Arc<TicketLease>,
	id: SubscriptionId,
	cursor: usize,
	values: Vec<Arc<T>>,
	_type: PhantomData<fn() -> T>,
}

/// Pending/contended/failed/stale requests never certify an empty region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketPoll {
	Pending,
	Ready,
	/// Session ended, or a published value was evicted before its snapshot.
	Stale,
	/// A scheme panicked. Partial output is deliberately withheld.
	Failed,
}

pub(super) fn prune_cancelled(state: &mut DemandState) {
	let before = state.subscriptions.len();
	state.subscriptions.retain(|_, subscription| {
		subscription.ticket.is_none() || !subscription.cancelled.load(Ordering::Acquire)
	});
	state.sweep_needed |= before != state.subscriptions.len();
}

impl HcsgDemand {
	/// Request one bounded region using the existing worker and storage.
	/// Exact scheme/region/class duplicates share generation and retention.
	/// Busy means mutex contention; Capacity means the bounded request pool
	/// is full. Callers retain their probe and retry without treating it as empty.
	/// Nonfinite or inverted bounds return InvalidRegion.
	pub fn try_request<T: GenerationScheme>(
		&self,
		region: Aabb3d,
		class: HcsgClass,
	) -> Result<HcsgTicket<T>, TicketRequestError> {
		if !region.min.is_finite() || !region.max.is_finite() || !region.min.cmple(region.max).all()
		{
			return Err(TicketRequestError::InvalidRegion);
		}
		let mut state = self.try_lock()?;
		prune_cancelled(&mut state);
		let type_id = TypeId::of::<T>();
		for (id, subscription) in &state.subscriptions {
			if let Some((kind, lease)) = &subscription.ticket {
				if *kind == type_id
					&& subscription.regions == [region]
					&& subscription.class == class
				{
					if let Some(lease) = lease.upgrade() {
						return Ok(HcsgTicket {
							lease,
							id: *id,
							cursor: 0,
							values: Vec::new(),
							_type: PhantomData,
						});
					}
				}
			}
		}
		if state.subscriptions.values().filter(|s| s.ticket.is_some()).count()
			>= MAX_TICKET_REQUESTS
		{
			return Err(TicketRequestError::Capacity);
		}
		let id = SubscriptionId::new(self.0.next_id.fetch_add(1, Ordering::Relaxed));
		let cancelled = Arc::new(AtomicBool::new(false));
		let lease = Arc::new(TicketLease { demand: self.clone(), cancelled: cancelled.clone() });
		let reach = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
		lock_mutex(&reach).insert(type_id);
		let pass = min_pass(&state);
		#[cfg(feature = "generation-tracing")]
		let trace = crate::hcsg::trace::JobTrace::queued(
			"journey-poi",
			std::any::type_name::<T>(),
			state.epoch,
		);
		#[cfg(feature = "generation-tracing")]
		let enqueue = trace.clone();
		state.subscriptions.insert(
			id,
			Subscription {
				regions: vec![region],
				focus: None,
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
				ticket: Some((type_id, Arc::downgrade(&lease))),
				cancelled,
				reach,
				#[cfg(feature = "generation-tracing")]
				trace,
			},
		);
		state.sweep_needed = true;
		self.0.wake.notify_all();
		drop(state);
		#[cfg(feature = "generation-tracing")]
		if let Some(trace) = enqueue {
			trace.emit_enqueue(id.trace_raw());
		}
		Ok(HcsgTicket { lease, id, cursor: 0, values: Vec::new(), _type: PhantomData })
	}
}

impl<T: GenerationScheme> HcsgTicket<T> {
	/// Available published snapshot after a successful Pending or Ready poll.
	/// Ready separately certifies that this request has finished. Busy, Stale
	/// and Failed do not validate the snapshot for the current session.
	pub fn values(&self) -> &[Arc<T>] {
		&self.values
	}

	/// Clone at most 32 published values per poll, using only try-lock reads.
	/// The ticket holds each completed Arc until its consumer releases it.
	pub fn try_poll(&mut self, storage: &HcsgStorage) -> Result<TicketPoll, Busy> {
		if self.lease.cancelled.load(Ordering::Acquire) {
			return Ok(TicketPoll::Stale);
		}
		let demand = &self.lease.demand;
		let Some(published) = demand.try_read(self.id, self.cursor)? else {
			return Ok(TicketPoll::Stale);
		};
		let ids = published.ids;
		let done = published.done;
		let failed = published.failed;
		if failed {
			return Ok(TicketPoll::Failed);
		}
		let entries = storage.try_entries::<T>(&ids)?;
		// Session cancellation can race the unlocked storage snapshot phase.
		if self.lease.cancelled.load(Ordering::Acquire) {
			return Ok(TicketPoll::Stale);
		}
		if entries.iter().any(Option::is_none) {
			return Ok(TicketPoll::Stale);
		}
		self.cursor += ids.len();
		self.values.extend(entries.into_iter().flatten().map(|entry| entry.value));
		if done {
			Ok(TicketPoll::Ready)
		} else {
			Ok(TicketPoll::Pending)
		}
	}
}

#[cfg(test)]
mod tests;
