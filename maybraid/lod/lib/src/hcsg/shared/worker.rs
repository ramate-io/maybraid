//! [`HcsgWorker`]: the thread that fills [`HcsgDemand`] subscriptions.

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};

use bevy::log::error;
use bevy::math::Vec3A;
use bevy::prelude::Resource;

use crate::gen::Id;

use super::context::GenerationContext;
use super::demand::{HcsgDemand, Job};
use super::storage::HcsgStorage;

/// Fills the newest unfinished subscription: discovers its ids, nearest the
/// focus first, generates what is missing, and appends every id whose value
/// is available. Stops early when the subscription is cancelled.
///
/// Dropping the worker shuts the thread down and joins it.
#[derive(Resource)]
pub struct HcsgWorker {
	demand: HcsgDemand,
	thread: Option<JoinHandle<()>>,
}

impl HcsgWorker {
	pub fn spawn(storage: HcsgStorage, demand: HcsgDemand) -> std::io::Result<Self> {
		let thread = thread::Builder::new().name("hcsg-worker".into()).spawn({
			let demand = demand.clone();
			move || run(&storage, &demand)
		})?;
		Ok(Self { demand, thread: Some(thread) })
	}
}

impl Drop for HcsgWorker {
	fn drop(&mut self) {
		self.demand.shutdown();
		if let Some(thread) = self.thread.take() {
			let _ = thread.join();
		}
	}
}

fn run(storage: &HcsgStorage, demand: &HcsgDemand) {
	while let Some(job) = demand.next_job() {
		if panic::catch_unwind(AssertUnwindSafe(|| fill(storage, demand, &job))).is_err() {
			error!("hcsg worker: generation panicked; subscription {:?} left partial", job.id);
		}
		demand.finish(&job);
	}
}

fn fill(storage: &HcsgStorage, demand: &HcsgDemand, job: &Job) {
	let stale = || job.cancelled.load(Ordering::Acquire);
	let mut cx = GenerationContext::with_stale(storage, &stale);
	let mut ids = (job.discover)(&mut cx, job.bounds);
	if let Some(focus) = job.focus {
		let focus = Vec3A::from(focus);
		let distance = |id: &Id| {
			id.origin_cell_bounds()
				.map_or(f32::INFINITY, |cell| ((cell.min + cell.max) * 0.5).distance_squared(focus))
		};
		ids.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
	}
	for id in ids {
		if stale() {
			return;
		}
		if (job.generate)(&mut cx, id) {
			demand.append(job.id, id);
		}
	}
}
