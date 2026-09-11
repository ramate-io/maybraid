//! Shared active-job census for LOD generate, present, and chunk fulfill.
//!
//! Queues are typed, so a first-load gate cannot query every `T` at once.
//! Each enqueue / pending-root add takes a ticket; pop, expire, and remove
//! release it. The count is approximate: saturating subtract so a missed
//! begin cannot wrap the counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

/// Process-wide count of in-flight LOD jobs.
///
/// The inner [`AtomicU64`] is shareable if generate later moves off-thread.
/// [`App::init_resource`] is idempotent — generate, present, and refresh
/// each ensure this exists.
#[derive(Resource, Clone, Debug)]
pub struct LodJobCounter {
	inner: Arc<AtomicU64>,
}

impl Default for LodJobCounter {
	fn default() -> Self {
		Self { inner: Arc::new(AtomicU64::new(0)) }
	}
}

impl LodJobCounter {
	pub fn begin(&self) {
		self.begin_n(1);
	}

	pub fn begin_n(&self, n: u64) {
		if n == 0 {
			return;
		}
		self.inner.fetch_add(n, Ordering::AcqRel);
	}

	pub fn end(&self) {
		self.end_n(1);
	}

	pub fn end_n(&self, n: u64) {
		if n == 0 {
			return;
		}
		loop {
			let cur = self.inner.load(Ordering::Acquire);
			let next = cur.saturating_sub(n);
			if self
				.inner
				.compare_exchange(cur, next, Ordering::AcqRel, Ordering::Acquire)
				.is_ok()
			{
				return;
			}
		}
	}

	pub fn active(&self) -> u64 {
		self.inner.load(Ordering::Acquire)
	}

	pub fn is_quiet(&self, threshold: u64) -> bool {
		self.active() <= threshold
	}
}

/// Idempotent: first plugin to run owns the default zeroed counter.
pub fn ensure_lod_job_counter(app: &mut App) {
	app.init_resource::<LodJobCounter>();
}

/// [`crate::LodLevelRootPending`] add hook. No-op when the resource is absent
/// so LOD unit tests can spawn pending roots without the counter.
pub(crate) fn count_lod_job_add(world: DeferredWorld, _ctx: HookContext) {
	if let Some(jobs) = world.get_resource::<LodJobCounter>() {
		jobs.begin();
	}
}

/// [`crate::LodLevelRootPending`] remove / despawn hook.
pub(crate) fn count_lod_job_remove(world: DeferredWorld, _ctx: HookContext) {
	if let Some(jobs) = world.get_resource::<LodJobCounter>() {
		jobs.end();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::LodLevelRootPending;

	#[test]
	fn begin_end_tracks_active() {
		let jobs = LodJobCounter::default();
		assert_eq!(jobs.active(), 0);
		assert!(jobs.is_quiet(0));
		jobs.begin();
		jobs.begin_n(2);
		assert_eq!(jobs.active(), 3);
		assert!(!jobs.is_quiet(0));
		jobs.end();
		assert_eq!(jobs.active(), 2);
		jobs.end_n(5);
		assert_eq!(jobs.active(), 0);
	}

	#[test]
	fn pending_root_hooks_count_when_resource_exists() {
		let mut app = App::new();
		app.init_resource::<LodJobCounter>();
		let entity = app.world_mut().spawn(LodLevelRootPending).id();
		assert_eq!(app.world().resource::<LodJobCounter>().active(), 1);
		app.world_mut().entity_mut(entity).despawn();
		assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
	}

	#[test]
	fn pending_root_hooks_are_silent_without_counter() {
		let mut app = App::new();
		let entity = app.world_mut().spawn(LodLevelRootPending).id();
		app.world_mut().entity_mut(entity).despawn();
	}
}
