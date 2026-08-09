//! Small helpers shared by begin / drain / complete.

use std::time::Instant;

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::types::{LodCullInFlight, LodSceneHostStreamed};

pub(super) fn ms(start: Instant) -> f64 {
	start.elapsed().as_secs_f64() * 1000.0
}

pub(super) fn roots_bag_entity(
	host_children: &Children,
	level_roots_heads: &Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
) -> Option<Entity> {
	for child in host_children.iter() {
		if level_roots_heads.contains(child) {
			return Some(child);
		}
	}
	None
}

/// True when the host already has any non-culling level root (ready **or** pending).
///
/// Pending siblings count as present so Medium→High (and similar) upgrades are
/// classified **warm** / Level rather than Presence — otherwise a paused mid-fill
/// Medium root would push High into the cold Presence queue and starve it.
pub(super) fn has_present_root(
	root_children: &Children,
	root_keys: &Query<&LodLevelRoot>,
	cull_inflight: &Query<(), With<LodCullInFlight>>,
) -> bool {
	for child in root_children.iter() {
		if root_keys.get(child).is_err() {
			continue;
		}
		if cull_inflight.contains(child) {
			continue;
		}
		return true;
	}
	false
}

/// Host entity for a level-root (`root → LodLevelRoots → host`).
pub(super) fn host_entity_for_root(root: Entity, child_of: &Query<&ChildOf>) -> Option<Entity> {
	let bag = child_of.get(root).ok()?.parent();
	Some(child_of.get(bag).ok()?.parent())
}

/// Host [`LodSceneLevel`] for a level-root entity (`root → LodLevelRoots → host`).
pub(super) fn host_desired_for_root(
	root: Entity,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
) -> Option<LodSceneLevel> {
	let host = host_entity_for_root(root, child_of)?;
	host_levels.get(host).ok().copied()
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

/// Next-level nested hosts under a level root are ready when `expected` streamed
/// hosts are present (mesh-only roots complete immediately).
pub(super) fn nested_hosts_streamed(
	root: Entity,
	expected: usize,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
) -> bool {
	let Ok(children) = children_q.get(root) else {
		return expected == 0;
	};

	let mut streamed = 0usize;
	let mut saw_host = false;
	for child in children.iter() {
		let Some(is_streamed) = next_level_host_streamed(child, children_q, hosts, streamed_hosts)
		else {
			continue;
		};
		saw_host = true;
		if is_streamed {
			streamed += 1;
		}
	}

	if !saw_host {
		return true;
	}
	streamed >= expected
}
