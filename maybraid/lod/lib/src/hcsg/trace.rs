//! Feature-only metadata on the existing subscription, not a second job queue.

use lod_generation_diagnostics::{enabled, nanos, Correlation};
use std::time::Instant;

#[derive(Clone)]
pub(super) struct JobTrace {
	pub origin: &'static str,
	pub scheme: &'static str,
	pub epoch: u64,
	pub quantum: u64,
	queued: Instant,
	pub queue_wait_ns: u64,
}
impl JobTrace {
	pub fn queued(origin: &'static str, scheme: &'static str, epoch: u64) -> Option<Self> {
		if !enabled() {
			return None;
		}
		Some(Self { origin, scheme, epoch, quantum: 0, queued: Instant::now(), queue_wait_ns: 0 })
	}
	/// Emit only after the demand guard is released. `queued` retains the
	/// enqueue timestamp captured while the subscription became eligible.
	pub fn emit_enqueue(&self, subscription: u64) {
		lod_generation_diagnostics::enqueue(
			self.origin,
			self.scheme,
			subscription,
			self.epoch,
			nanos(self.queued.elapsed()),
		);
	}
	pub fn pick(&mut self) {
		self.quantum += 1;
		self.queue_wait_ns = nanos(self.queued.elapsed());
	}
	pub fn requeue(&mut self) {
		self.queued = Instant::now();
	}
	pub fn correlation(&self, subscription: u64) -> Correlation {
		Correlation {
			subscription,
			epoch: self.epoch,
			quantum: self.quantum,
			queue_wait_ns: self.queue_wait_ns,
			epoch_known: true,
			application_session: 0,
		}
	}
}
