//! Impulse-queue liveness: pending generate/present ids die when they leave keep.

use std::collections::VecDeque;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;

use super::Id;

/// Default XZ slack on [`crate::LodGenerateKeepRegion`] / [`crate::LodPresentKeepRegion`].
///
/// Override per channel in the world. A 100 m tile-cross should not drop cells
/// still on the ring edge.
pub const QUEUE_KEEP_SLACK_XZ: f32 = 100.0;

/// Expand `keep` on XZ only (Y is not the live axis for grove/forest rings).
pub fn expand_keep_xz(keep: Aabb3d, slack: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(keep.min.x - slack, keep.min.y, keep.min.z - slack),
		Vec3::new(keep.max.x + slack, keep.max.y, keep.max.z + slack),
	)
}

fn intersects_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Whether `id` should stay queued. No origin cell → keep (cannot test).
pub fn id_lives_in_keep(id: Id, keep: Aabb3d, slack: f32) -> bool {
	match id.origin_cell_bounds() {
		None => true,
		Some(bounds) => intersects_xz(expand_keep_xz(keep, slack), bounds),
	}
}

/// Viewer-to-origin-cell XZ distance squared (generate / present drain sort).
pub fn id_xz_distance2(id: Id, origin: Vec3) -> f32 {
	let Some(bounds) = id.origin_cell_bounds() else {
		return f32::MAX;
	};
	let center = (bounds.min + bounds.max) * 0.5;
	let dx = center.x - origin.x;
	let dz = center.z - origin.z;
	dx * dx + dz * dz
}

/// True when the keep AABB appeared, vanished, or moved.
pub fn keep_region_changed(previous: Option<Aabb3d>, current: Option<Aabb3d>) -> bool {
	match (previous, current) {
		(Some(a), Some(b)) => !keep_regions_match(a, b),
		(None, None) => false,
		_ => true,
	}
}

fn keep_regions_match(a: Aabb3d, b: Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

/// Drop pending origin ids whose cell sits outside keep + `slack`.
///
/// No keep AABB → no expiry (nothing is known to be live).
pub fn expire_pending_outside_keep(pending: &mut VecDeque<Id>, keep: Option<Aabb3d>, slack: f32) {
	let Some(keep) = keep else {
		return;
	};
	pending.retain(|id| id_lives_in_keep(*id, keep, slack));
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gen::tests::test_utils::cell;

	#[test]
	fn slack_keeps_a_cell_one_tile_behind_a_jump() {
		let keep = cell(100.0);
		assert!(id_lives_in_keep(Id::from_cell(cell(0.0)), keep, QUEUE_KEEP_SLACK_XZ));
		assert!(!id_lives_in_keep(Id::from_cell(cell(250.0)), keep, QUEUE_KEEP_SLACK_XZ));
	}

	#[test]
	fn no_keep_does_not_expire() {
		let mut pending = VecDeque::from([Id::from_cell(cell(0.0)), Id::from_cell(cell(200.0))]);
		expire_pending_outside_keep(&mut pending, None, QUEUE_KEEP_SLACK_XZ);
		assert_eq!(pending.len(), 2);
	}

	#[test]
	fn expire_drops_only_outside_slack() {
		let mut pending = VecDeque::from([Id::from_cell(cell(0.0)), Id::from_cell(cell(250.0))]);
		expire_pending_outside_keep(&mut pending, Some(cell(100.0)), QUEUE_KEEP_SLACK_XZ);
		assert_eq!(pending, VecDeque::from([Id::from_cell(cell(0.0))]));
	}

	#[test]
	fn expire_honors_custom_slack() {
		let mut pending = VecDeque::from([Id::from_cell(cell(250.0))]);
		expire_pending_outside_keep(&mut pending, Some(cell(100.0)), 200.0);
		assert_eq!(pending, VecDeque::from([Id::from_cell(cell(250.0))]));
		expire_pending_outside_keep(&mut pending, Some(cell(100.0)), 0.0);
		assert!(pending.is_empty());
	}

	#[test]
	fn bytes_id_without_origin_stays() {
		let id = Id::Bytes(crate::gen::Bytes([0; 32]));
		assert!(id_lives_in_keep(id, cell(0.0), QUEUE_KEEP_SLACK_XZ));
	}

	#[test]
	fn keep_region_change_ignores_identical_aabb() {
		assert!(!keep_region_changed(Some(cell(0.0)), Some(cell(0.0))));
		assert!(keep_region_changed(None, Some(cell(0.0))));
		assert!(keep_region_changed(Some(cell(0.0)), Some(cell(100.0))));
	}
}
