//! Warm-swap completion once content + nested hosts are Streamed.

use std::time::Instant;

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use super::types::{
	LodChunkFulfillment, LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed,
	LodWantsCull,
};
use super::util::{ms, nested_hosts_streamed};

/// Finish pending roots that are content-[`LodLevelRootStreamed`] and whose nested
/// hosts are [`LodSceneHostStreamed`]: clear pending, show root, hide siblings.
pub fn complete_chunk_lod_fulfill(
	mut commands: Commands,
	pending: Query<
		(Entity, Option<&LodChunkFulfillment>, Option<&ChildOf>, Has<LodLevelRootStreamed>),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
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
	for (root_entity, fulfillment, root_child_of, content_streamed) in &pending {
		let expected = match fulfillment {
			Some(f) if !f.is_content_complete() => continue,
			Some(f) => f.expected,
			None => 0,
		};
		if !content_streamed {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		if !nested_hosts_streamed(
			root_entity,
			expected,
			&children_q,
			&nested_hosts,
			&streamed_hosts,
		) {
			waiting_nested += 1;
			continue;
		}

		commands
			.entity(root_entity)
			.remove::<LodChunkFulfillment>()
			.remove::<LodLevelRootPending>();
		if let Ok(mut vis) = visibilities.get_mut(root_entity) {
			*vis = Visibility::Inherited;
		}
		completed += 1;

		let Some(root_child_of) = root_child_of else {
			continue;
		};
		let roots_bag = root_child_of.0;
		if let Ok(host_of) = child_of.get(roots_bag) {
			commands.entity(host_of.0).insert(LodSceneHostStreamed);
		}

		let Ok(siblings) = level_roots_heads.get(roots_bag) else {
			continue;
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
	if completed > 0 || waiting_nested > 0 {
		info!(
			"[lod.chunk] complete: {completed} roots swapped, {waiting_nested} waiting nested \
			 in {:.2}ms",
			ms(t0)
		);
	}
}
