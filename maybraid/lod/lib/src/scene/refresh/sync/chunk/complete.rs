//! Warm-swap completion once content + nested hosts are Streamed.

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRootOverlap, LodSceneHost};

use super::types::{
	LodChunkFulfillBudget, LodChunkFulfillment, LodCullInFlight, LodLazyPending,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed,
};
use super::util::{count_nested_hosts, subtree_has_lod_lazy_pending};

/// Finish pending roots that are content-[`LodLevelRootStreamed`] and whose nested
/// hosts are [`LodSceneHostStreamed`]: clear pending and show the root.
///
/// Sibling hide is deferred one frame ([`LodLevelRootOverlap`]) so extract still
/// has last frame's meshes. Nested readiness uses
/// [`LodChunkFulfillment::nested_streamed`] /
/// [`LodChunkFulfillment::nested_required`] (initialized once at content-complete).
/// A descendant [`LodLazyPending`] also blocks the swap until kit mesh + material
/// exist. Visibility swaps are capped by
/// [`LodChunkFulfillBudget::completes_per_frame`]; Streamed / nested bookkeeping
/// still runs so parent jobs can catch up.
pub fn complete_chunk_lod_fulfill(
	mut commands: Commands,
	mut pending: Query<
		(Entity, Option<&mut LodChunkFulfillment>, Option<&ChildOf>, Has<LodLevelRootStreamed>),
		(With<LodLevelRootPending>, Without<LodCullInFlight>),
	>,
	children_q: Query<&Children>,
	nested_hosts: Query<(), With<LodSceneHost>>,
	streamed_hosts: Query<(), With<LodSceneHostStreamed>>,
	lazy_pending: Query<(), With<LodLazyPending>>,
	child_of: Query<&ChildOf>,
	mut visibilities: Query<&mut Visibility>,
	budget: Res<LodChunkFulfillBudget>,
) {
	let mut remaining = budget.completes_per_frame;
	for (root_entity, fulfillment, root_child_of, content_streamed) in &mut pending {
		let Some(mut fulfillment) = fulfillment else {
			// Pending without a plan — treat as content-complete mesh placeholder.
			if !content_streamed {
				commands.entity(root_entity).insert(LodLevelRootStreamed);
			}
			if subtree_has_lod_lazy_pending(root_entity, &children_q, &lazy_pending) {
				continue;
			}
			if remaining == 0 {
				continue;
			}
			remaining -= 1;
			commands.entity(root_entity).remove::<LodLevelRootPending>();
			finish_root(&mut commands, root_entity, root_child_of, &child_of, &mut visibilities);
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
			continue;
		}
		if subtree_has_lod_lazy_pending(root_entity, &children_q, &lazy_pending) {
			continue;
		}
		if remaining == 0 {
			continue;
		}
		remaining -= 1;

		commands
			.entity(root_entity)
			.remove::<LodChunkFulfillment>()
			.remove::<LodLevelRootPending>();
		finish_root(&mut commands, root_entity, root_child_of, &child_of, &mut visibilities);
	}
}

fn finish_root(
	commands: &mut Commands,
	root_entity: Entity,
	root_child_of: Option<&ChildOf>,
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
		commands
			.entity(host_of.0)
			.insert(LodSceneHostStreamed)
			.insert(LodLevelRootOverlap);
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::scene::refresh::sync::chunk::types::LodChunkFulfillBudget;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<LodChunkFulfillBudget>()
			.add_systems(Update, complete_chunk_lod_fulfill);
		app
	}

	#[test]
	fn complete_waits_for_lazy_descendant() {
		let mut app = app();
		let root = app
			.world_mut()
			.spawn((LodLevelRootPending, Visibility::Hidden, Transform::default()))
			.id();
		let lazy = app.world_mut().spawn((LodLazyPending, ChildOf(root))).id();
		app.update();
		assert!(app.world().get::<LodLevelRootPending>(root).is_some());

		app.world_mut().entity_mut(lazy).remove::<LodLazyPending>();
		app.update();
		assert!(app.world().get::<LodLevelRootPending>(root).is_none());
	}
}
