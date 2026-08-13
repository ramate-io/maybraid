//! Warm-swap completion once content + nested hosts are Streamed.

use std::time::Instant;

use bevy::prelude::*;

use crate::lod_chunk_trace;
use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use super::types::{
	LodChunkFulfillment, LodCullInFlight, LodLevelRootPending, LodLevelRootStreamed,
	LodSceneHostStreamed,
};
use super::util::{count_nested_hosts, ms};

/// Finish pending roots that are content-[`LodLevelRootStreamed`] and whose nested
/// hosts are [`LodSceneHostStreamed`]: clear pending, show root, hide siblings.
///
/// Nested readiness uses [`LodChunkFulfillment::nested_streamed`] /
/// [`LodChunkFulfillment::nested_required`] (initialized once at content-complete).
pub fn complete_chunk_lod_fulfill(
	mut commands: Commands,
	mut pending: Query<
		(Entity, Option<&mut LodChunkFulfillment>, Option<&ChildOf>, Has<LodLevelRootStreamed>),
		(With<LodLevelRootPending>, Without<LodCullInFlight>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	children_q: Query<&Children>,
	nested_hosts: Query<(), With<LodSceneHost>>,
	streamed_hosts: Query<(), With<LodSceneHostStreamed>>,
	root_keys: Query<&LodLevelRoot>,
	pending_marker: Query<(), With<LodLevelRootPending>>,
	child_of: Query<&ChildOf>,
	mut visibilities: Query<&mut Visibility>,
) {
	let t0 = Instant::now();
	let mut completed = 0u32;
	let mut waiting_nested = 0u32;
	for (root_entity, fulfillment, root_child_of, content_streamed) in &mut pending {
		let Some(mut fulfillment) = fulfillment else {
			// Pending without a plan — treat as content-complete mesh placeholder.
			if !content_streamed {
				commands.entity(root_entity).insert(LodLevelRootStreamed);
			}
			commands.entity(root_entity).remove::<LodLevelRootPending>();
			finish_root(
				&mut commands,
				root_entity,
				root_child_of,
				&level_roots_heads,
				&root_keys,
				&pending_marker,
				&child_of,
				&mut visibilities,
			);
			completed += 1;
			continue;
		};

		if !fulfillment.is_content_complete() {
			continue;
		}
		if !content_streamed {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		if fulfillment.nested_required.is_none() {
			let (required, streamed) =
				count_nested_hosts(root_entity, &children_q, &nested_hosts, &streamed_hosts);
			fulfillment.nested_required = Some(required);
			fulfillment.nested_streamed = fulfillment.nested_streamed.max(streamed);
		}
		if !fulfillment.nested_ready() {
			waiting_nested += 1;
			continue;
		}

		commands
			.entity(root_entity)
			.remove::<LodChunkFulfillment>()
			.remove::<LodLevelRootPending>();
		finish_root(
			&mut commands,
			root_entity,
			root_child_of,
			&level_roots_heads,
			&root_keys,
			&pending_marker,
			&child_of,
			&mut visibilities,
		);
		completed += 1;
	}
	if lod_chunk_trace() && (completed > 0 || waiting_nested > 0) {
		info!(
			"[lod.chunk] complete: {completed} roots swapped, {waiting_nested} waiting nested \
			 in {:.2}ms",
			ms(t0)
		);
	}
}

fn finish_root(
	commands: &mut Commands,
	root_entity: Entity,
	root_child_of: Option<&ChildOf>,
	level_roots_heads: &Query<&Children, With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	pending_marker: &Query<(), With<LodLevelRootPending>>,
	child_of: &Query<&ChildOf>,
	visibilities: &mut Query<&mut Visibility>,
) {
	if let Ok(mut vis) = visibilities.get_mut(root_entity) {
		*vis = Visibility::Inherited;
	}

	let Some(root_child_of) = root_child_of else {
		return;
	};
	let roots_bag = root_child_of.0;
	if let Ok(host_of) = child_of.get(roots_bag) {
		commands.entity(host_of.0).insert(LodSceneHostStreamed);
	}

	let Ok(siblings) = level_roots_heads.get(roots_bag) else {
		return;
	};
	for sibling in siblings.iter() {
		if sibling == root_entity {
			continue;
		}
		if root_keys.contains(sibling) || pending_marker.contains(sibling) {
			if let Ok(mut vis) = visibilities.get_mut(sibling) {
				*vis = Visibility::Hidden;
			}
		}
	}
}

/// When a nested host becomes [`LodSceneHostStreamed`], bump its enclosing pending root.
pub fn bump_nested_streamed_progress(
	added: Query<Entity, Added<LodSceneHostStreamed>>,
	child_of: Query<&ChildOf>,
	level_roots: Query<(), With<LodLevelRoot>>,
	mut fulfillments: Query<&mut LodChunkFulfillment, With<LodLevelRootPending>>,
) {
	for host in &added {
		let mut current = host;
		for _ in 0..32 {
			let Ok(of) = child_of.get(current) else {
				break;
			};
			let parent = of.parent();
			if level_roots.contains(parent) {
				if let Ok(mut job) = fulfillments.get_mut(parent) {
					job.nested_streamed = job.nested_streamed.saturating_add(1);
				}
				break;
			}
			current = parent;
		}
	}
}
