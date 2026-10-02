//! Shared helpers for budgeted generate/present drain loops.

use std::time::{Duration, Instant};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::debug;

/// True when `started.elapsed()` has reached `budget` (zero budget disables the limit).
pub(crate) fn time_up(started: Instant, budget: Duration) -> bool {
	!budget.is_zero() && started.elapsed() >= budget
}

/// Log when a single drain step exceeds `maximum` (zero maximum disables the check).
pub(crate) fn warn_atomic_overrun(
	domain: &'static str,
	stage: &'static str,
	elapsed: Duration,
	maximum: Duration,
) {
	if maximum.is_zero() || elapsed <= maximum {
		return;
	}
	debug!(
		stage,
		elapsed_us = elapsed.as_micros(),
		max_us = maximum.as_micros(),
		"LOD {domain} quantum exceeded max_atomic_cost"
	);
}

/// True when two keep/scan AABBs match on XZ bounds within 1 mm.
pub(crate) fn regions_match(a: Aabb3d, b: Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

/// True when `a` and `b` overlap on the XZ plane.
pub(crate) fn regions_overlap_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}
