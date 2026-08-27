//! Impulse-queue liveness: pending generate/present ids die when they leave keep.

use std::collections::VecDeque;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;

use super::Id;

/// XZ slack so a 100 m tile-cross does not drop cells still on the ring edge.
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

/// Drop pending origin ids whose cell sits outside keep + [`QUEUE_KEEP_SLACK_XZ`].
///
/// No keep AABB → no expiry (nothing is known to be live).
pub fn expire_pending_outside_keep(pending: &mut VecDeque<Id>, keep: Option<Aabb3d>) {
	let Some(keep) = keep else {
		return;
	};
	pending.retain(|id| id_lives_in_keep(*id, keep, QUEUE_KEEP_SLACK_XZ));
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
		expire_pending_outside_keep(&mut pending, None);
		assert_eq!(pending.len(), 2);
	}

	#[test]
	fn expire_drops_only_outside_slack() {
		let mut pending = VecDeque::from([Id::from_cell(cell(0.0)), Id::from_cell(cell(250.0))]);
		expire_pending_outside_keep(&mut pending, Some(cell(100.0)));
		assert_eq!(pending, VecDeque::from([Id::from_cell(cell(0.0))]));
	}

	#[test]
	fn bytes_id_without_origin_stays() {
		let id = Id::Bytes(crate::gen::Bytes([0; 32]));
		assert!(id_lives_in_keep(id, cell(0.0), QUEUE_KEEP_SLACK_XZ));
	}
}
