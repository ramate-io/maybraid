//! Retention planning and worker sweeps when the live set changes.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::PoisonError;

use bevy::math::bounding::Aabb3d;

use super::schedule::{pick_job, Job};
use super::state::{lock_mutex, DemandState};
use super::HcsgDemand;

/// What the worker should do next: sweep, fill a quantum, or stop.
pub(crate) enum WorkerWait {
	Job(Job),
	/// Regions to retain, keyed by the types live subscriptions reach.
	Sweep(HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>>),
	Shutdown,
}

pub(super) fn retention_plan(
	state: &DemandState,
) -> HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>> {
	let mut regions_by_type: HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>> = HashMap::new();
	for subscription in state.subscriptions.values() {
		for type_id in lock_mutex(&subscription.reach).iter() {
			regions_by_type
				.entry(*type_id)
				.or_default()
				.extend(subscription.regions.iter().copied());
		}
	}
	for regions in regions_by_type.values_mut() {
		dedupe_regions(regions);
	}
	regions_by_type
}

/// Collapses identical boxes from the union of live subscriptions. The
/// retention set is unchanged; sweeps and margin expansion do less work.
fn dedupe_regions(regions: &mut Vec<Aabb3d>) {
	if regions.len() <= 1 {
		return;
	}
	regions.sort_by(|a, b| {
		a.min
			.x
			.total_cmp(&b.min.x)
			.then_with(|| a.min.y.total_cmp(&b.min.y))
			.then_with(|| a.min.z.total_cmp(&b.min.z))
			.then_with(|| a.max.x.total_cmp(&b.max.x))
			.then_with(|| a.max.y.total_cmp(&b.max.y))
			.then_with(|| a.max.z.total_cmp(&b.max.z))
	});
	regions.dedup_by(|a, b| a.min == b.min && a.max == b.max);
}

#[cfg(test)]
mod dedupe_tests {
	use bevy::math::Vec3;

	use super::*;

	fn box_at(x: f32) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
	}

	#[test]
	fn dedupe_regions_collapses_identical_boxes() {
		let a = box_at(0.0);
		let b = box_at(1.0);
		let mut regions = vec![a, b, a, b, a];
		dedupe_regions(&mut regions);
		assert_eq!(regions, vec![a, b]);
	}
}

impl HcsgDemand {
	/// The next sweep or unfinished subscription. Blocks until there is one.
	/// Sweep is taken first whenever the live set has changed.
	pub(in crate::hcsg) fn next_work(&self) -> WorkerWait {
		let mut state = self.lock();
		loop {
			if state.shutdown {
				return WorkerWait::Shutdown;
			}
			if state.sweep_needed {
				state.sweep_needed = false;
				self.0.sweeps.fetch_add(1, Ordering::Relaxed);
				return WorkerWait::Sweep(retention_plan(&state));
			}
			if let Some(job) = pick_job(&mut state) {
				state.working = true;
				self.0.picks.fetch_add(1, Ordering::Relaxed);
				return WorkerWait::Job(job);
			}
			self.0.idle.notify_all();
			state = self.0.wake.wait(state).unwrap_or_else(PoisonError::into_inner);
		}
	}

	/// Applies a pending sweep plan without a worker. Tests use this after a
	/// mid-quantum replacement.
	#[cfg(test)]
	pub(crate) fn try_sweep(&self) -> Option<HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>>> {
		let mut state = self.lock();
		if !state.sweep_needed {
			return None;
		}
		state.sweep_needed = false;
		self.0.sweeps.fetch_add(1, Ordering::Relaxed);
		Some(retention_plan(&state))
	}
}
