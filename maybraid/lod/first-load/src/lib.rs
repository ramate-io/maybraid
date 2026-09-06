//! Small first-load activity signal based on startup quiescence.
//!
//! Producers hold [`FirstLoadPermit`]s while asynchronous or recursively
//! discovered work is outstanding. Consumers wait for a minimum observation
//! period and a quiet window below the configured low-water mark.

use std::sync::{
	atomic::{AtomicU64, Ordering},
	Arc,
};
use std::time::Duration;

use bevy::prelude::*;

#[derive(Debug, Default)]
struct SharedActivity {
	outstanding: AtomicU64,
	revision: AtomicU64,
	started: AtomicU64,
}

/// Process-wide first-load work signal. Clones share the same atomics.
#[derive(Resource, Clone, Debug, Default)]
pub struct FirstLoadActivity {
	shared: Arc<SharedActivity>,
}

impl FirstLoadActivity {
	/// Start one unit of work. Dropping the returned permit completes it.
	pub fn begin(&self) -> FirstLoadPermit {
		self.shared.outstanding.fetch_add(1, Ordering::AcqRel);
		self.shared.started.fetch_add(1, Ordering::AcqRel);
		self.shared.revision.fetch_add(1, Ordering::AcqRel);
		FirstLoadPermit { shared: Some(Arc::clone(&self.shared)) }
	}

	/// Record work discovery or visible progress that has no lifetime permit.
	pub fn pulse(&self) {
		self.shared.revision.fetch_add(1, Ordering::AcqRel);
	}

	pub fn snapshot(&self) -> FirstLoadSnapshot {
		FirstLoadSnapshot {
			outstanding: self.shared.outstanding.load(Ordering::Acquire),
			revision: self.shared.revision.load(Ordering::Acquire),
			started: self.shared.started.load(Ordering::Acquire),
		}
	}
}

/// A non-cloneable, idempotent work ticket.
#[derive(Component, Debug)]
pub struct FirstLoadPermit {
	shared: Option<Arc<SharedActivity>>,
}

impl Drop for FirstLoadPermit {
	fn drop(&mut self) {
		let Some(shared) = self.shared.take() else {
			return;
		};
		let previous = shared.outstanding.fetch_sub(1, Ordering::AcqRel);
		debug_assert!(previous > 0, "first-load permit underflow");
		shared.revision.fetch_add(1, Ordering::AcqRel);
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FirstLoadSnapshot {
	pub outstanding: u64,
	pub revision: u64,
	pub started: u64,
}

/// Startup stabilization thresholds.
#[derive(Resource, Debug, Clone, Copy)]
pub struct FirstLoadConfig {
	pub minimum_duration: Duration,
	pub settle_window: Duration,
	pub low_watermark: u64,
}

impl Default for FirstLoadConfig {
	fn default() -> Self {
		Self {
			minimum_duration: Duration::from_secs(1),
			settle_window: Duration::from_millis(500),
			low_watermark: 0,
		}
	}
}

/// Monotonic display progress plus the current quiescence decision.
#[derive(Resource, Debug, Clone)]
pub struct FirstLoadStatus {
	pub settled: bool,
	pub progress: f32,
	pub outstanding: u64,
	pub started: u64,
	pub quiet_for: Duration,
	observed_for: Duration,
	peak_outstanding: u64,
	last_revision: u64,
}

impl Default for FirstLoadStatus {
	fn default() -> Self {
		Self {
			settled: false,
			progress: 0.0,
			outstanding: 0,
			started: 0,
			quiet_for: Duration::ZERO,
			observed_for: Duration::ZERO,
			peak_outstanding: 0,
			last_revision: 0,
		}
	}
}

impl FirstLoadStatus {
	fn observe(&mut self, snapshot: FirstLoadSnapshot, delta: Duration, config: FirstLoadConfig) {
		self.observed_for = self.observed_for.saturating_add(delta);
		self.outstanding = snapshot.outstanding;
		self.started = snapshot.started;
		self.peak_outstanding = self.peak_outstanding.max(snapshot.outstanding);

		let changed = snapshot.revision != self.last_revision;
		self.last_revision = snapshot.revision;
		if changed || snapshot.outstanding > config.low_watermark {
			self.quiet_for = Duration::ZERO;
		} else {
			self.quiet_for = self.quiet_for.saturating_add(delta);
		}

		self.settled = snapshot.started > 0
			&& self.observed_for >= config.minimum_duration
			&& self.quiet_for >= config.settle_window;

		let drained = if self.peak_outstanding == 0 {
			0.0
		} else {
			1.0 - snapshot.outstanding as f32 / self.peak_outstanding as f32
		};
		let quiet = if config.settle_window.is_zero() {
			1.0
		} else {
			self.quiet_for.as_secs_f32() / config.settle_window.as_secs_f32()
		};
		let candidate = if self.settled {
			1.0
		} else {
			(0.1 + 0.75 * drained + 0.14 * quiet.clamp(0.0, 1.0)).min(0.99)
		};
		self.progress = self.progress.max(candidate);
	}
}

pub struct FirstLoadPlugin;

impl Plugin for FirstLoadPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<FirstLoadActivity>()
			.init_resource::<FirstLoadConfig>()
			.init_resource::<FirstLoadStatus>()
			.add_systems(Last, update_first_load_status);
	}
}

fn update_first_load_status(
	time: Res<Time<Real>>,
	activity: Res<FirstLoadActivity>,
	config: Res<FirstLoadConfig>,
	mut status: ResMut<FirstLoadStatus>,
) {
	status.observe(activity.snapshot(), time.delta(), *config);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn settling_requires_started_work_and_a_quiet_window() {
		let config = FirstLoadConfig {
			minimum_duration: Duration::from_secs(1),
			settle_window: Duration::from_millis(500),
			low_watermark: 0,
		};
		let activity = FirstLoadActivity::default();
		let mut status = FirstLoadStatus::default();
		status.observe(activity.snapshot(), Duration::from_secs(2), config);
		assert!(!status.settled);

		let permit = activity.begin();
		status.observe(activity.snapshot(), Duration::from_secs(1), config);
		assert!(!status.settled);
		drop(permit);
		status.observe(activity.snapshot(), Duration::from_millis(250), config);
		status.observe(activity.snapshot(), Duration::from_millis(500), config);
		assert!(status.settled);
		assert_eq!(status.progress, 1.0);
	}
}
