//! Small helpers shared by begin / drain / complete.

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodSceneHost};

use super::types::{LodCullInFlight, LodSceneHostStreamed};

/// True when the host already has any non-culling level root (ready **or** pending).
///
/// Pending siblings count as present so Medium→High (and similar) upgrades are
/// classified **warm** / Desired rather than Presence — otherwise a paused mid-fill
/// Medium root would push High into the cold Presence queue and starve it.
pub(super) fn has_present_root(
	root_children: &Children,
	root_keys: &Query<&LodLevelRoot>,
	cull_inflight: &Query<(), With<LodCullInFlight>>,
) -> bool {
	root_children
		.iter()
		.any(|child| root_keys.contains(child) && !cull_inflight.contains(child))
}

/// Whether `entity` is a [`LodSceneHost`], or wraps one as a direct child.
///
/// Returns `Some(streamed)` when a next-level host is found.
fn next_level_host_streamed(
	entity: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
) -> Option<bool> {
	if hosts.contains(entity) {
		return Some(streamed_hosts.contains(entity));
	}
	let Ok(kids) = children_q.get(entity) else {
		return None;
	};
	for kid in kids.iter() {
		if hosts.contains(kid) {
			return Some(streamed_hosts.contains(kid));
		}
	}
	None
}

/// Count next-level nested hosts under a level root and how many are streamed.
///
/// Returns `(required, streamed)`. `required == 0` when there are no nested hosts
/// (mesh-only roots complete immediately).
pub(super) fn count_nested_hosts(
	root: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
) -> (usize, usize) {
	let Ok(children) = children_q.get(root) else {
		return (0, 0);
	};

	let mut required = 0usize;
	let mut streamed = 0usize;
	for child in children.iter() {
		let Some(is_streamed) = next_level_host_streamed(child, children_q, hosts, streamed_hosts)
		else {
			continue;
		};
		required += 1;
		if is_streamed {
			streamed += 1;
		}
	}
	(required, streamed)
}
