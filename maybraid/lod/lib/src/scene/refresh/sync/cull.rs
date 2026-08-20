//! Budgeted level-root / host teardown per [`LodScene::scene_lod_culls`].
//!
//! Unwanted roots are enqueued via [`LodCullRequest`] + [`LodCullInFlight`] (not
//! hard-despawned). [`drain_lod_cull`] waits on **next-level** nested hosts /
//! bag roots (same shallow scan as fulfill streaming), then recursive-despawns
//! a ready entity in one command under [`super::chunk::LodChunkBudgetClock`].

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
};
use crate::scene::chunk::DEFAULT_CHUNK_WEIGHT;
use crate::scene::cull::LodSceneCulls;
use crate::scene::host::{
	lod_level_roots_entity, nested_host_parent_allows_refresh, LodLevelRoot, LodLevelRoots,
	LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::chunk::{
	LodChunkBudgetClock, LodChunkFulfillBudget, LodChunkFulfillment, LodCullInFlight,
	LodLevelRootPending,
};

/// Impulse: tear down `entity` under budgeted cull ([`LodCullInFlight`]).
///
/// Applied by [`apply_lod_cull_requests`]. Enqueue helpers may also insert
/// [`LodCullInFlight`] directly so the next chained system sees it after
/// `ApplyDeferred`. Not used for pausing not-desired fulfill jobs.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodCullRequest {
	pub entity: Entity,
}

/// Insert [`LodCullInFlight`] from [`LodCullRequest`] messages (idempotent).
pub fn apply_lod_cull_requests(
	mut commands: Commands,
	mut reader: MessageReader<LodCullRequest>,
	existing: Query<(), With<LodCullInFlight>>,
) {
	for LodCullRequest { entity } in reader.read() {
		if existing.contains(*entity) {
			continue;
		}
		if let Ok(mut entity_commands) = commands.get_entity(*entity) {
			entity_commands.insert((LodCullInFlight { started: false }, Visibility::Hidden));
		}
	}
}

/// Enqueue budgeted cull: message + component (hidden).
pub fn enqueue_lod_cull(
	commands: &mut Commands,
	writer: &mut MessageWriter<LodCullRequest>,
	entity: Entity,
	already: &Query<(), With<LodCullInFlight>>,
) {
	if already.contains(entity) {
		return;
	}
	writer.write(LodCullRequest { entity });
	commands
		.entity(entity)
		.insert((LodCullInFlight { started: false }, Visibility::Hidden));
}

/// Mark inactive [`LodLevelRoot`]s for budgeted cull per [`LodScene::scene_lod_culls`].
///
/// Never targets the host's current [`LodSceneLevel`]. Hidden roots not listed
/// stay warm for cheap band flips. Skips the host while any **active** (not
/// already culling) level root is still [`LodLevelRootPending`] so warm-hold
/// roots are not GC'd mid-swap.
pub fn cull_lod_level_roots<T, FHost, FNode>(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullRequest>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, FNode)>,
	hosts: Query<(Entity, &T, &LodSceneLevel), (With<LodSceneHost>, FHost)>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
	wants_cull: Query<(), With<LodCullInFlight>>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	children_q: Query<&Children>,
	visibilities: Query<&Visibility>,
) where
	T: Component + LodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	if snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&snapshots);

	for (host, scene, current) in &hosts {
		if !nested_host_parent_allows_refresh(
			host,
			&child_of,
			&host_levels,
			&root_keys,
			&children_q,
			&level_roots_bags,
			&visibilities,
		) {
			continue;
		}
		// Viewer-only ref (no per-host dominant level vote).
		let Some(lod_ref) = refs.first() else {
			continue;
		};
		let culls = scene.scene_lod_culls(lod_ref, *current);
		if matches!(culls, LodSceneCulls::None) {
			continue;
		}

		let Ok(host_children) = children_q.get(host) else {
			continue;
		};
		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_bags) else {
			continue;
		};
		let Ok(root_children) = children_q.get(roots_entity) else {
			continue;
		};

		// Active warm-swap pending (not already tearing down).
		if root_children
			.iter()
			.any(|child| pending.contains(child) && !wants_cull.contains(child))
		{
			continue;
		}

		for child in root_children.iter() {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if root.0 == *current {
				continue;
			}
			if wants_cull.contains(child) {
				continue;
			}
			if culls.should_cull(root.0) {
				enqueue_lod_cull(&mut commands, &mut cull_writer, child, &wants_cull);
			}
		}
	}
}

/// Next-level [`LodSceneHost`]s under `root` (direct child, or one pose hop).
///
/// Same contract as fulfill streaming (`count_nested_hosts`) — does not DFS kits.
fn shallow_nested_hosts(
	root: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
) -> Vec<Entity> {
	let Ok(children) = children_q.get(root) else {
		return Vec::new();
	};
	let mut out = Vec::new();
	for child in children.iter() {
		if hosts.contains(child) {
			if child != root {
				out.push(child);
			}
			continue;
		}
		let Ok(kids) = children_q.get(child) else {
			continue;
		};
		for kid in kids.iter() {
			if hosts.contains(kid) && kid != root {
				out.push(kid);
			}
		}
	}
	out
}

/// [`LodLevelRoot`]s in this host's [`LodLevelRoots`] bag (not a content DFS).
fn bag_level_roots(
	host: Entity,
	children_q: &Query<&Children>,
	bags: &Query<(), With<LodLevelRoots>>,
	level_roots: &Query<(), With<LodLevelRoot>>,
) -> Vec<Entity> {
	let Ok(host_children) = children_q.get(host) else {
		return Vec::new();
	};
	let Some(bag) = lod_level_roots_entity(host_children, bags) else {
		return Vec::new();
	};
	let Ok(roots) = children_q.get(bag) else {
		return Vec::new();
	};
	roots.iter().filter(|child| level_roots.contains(*child)).collect()
}

/// Budgeted teardown for [`LodCullInFlight`] entities.
///
/// - Level root: enqueue next-level nested hosts (shallow) and wait.
/// - [`LodSceneHost`]: enqueue bag [`LodLevelRoot`]s and wait.
/// - Ready: recursive-despawn the entity when spawned/child weight fits
///   [`LodChunkBudgetClock::cull_remaining`], or when a
///   [`LodChunkFulfillBudget::cull_root_despawns_per_frame`] slot remains.
///   Already-[`Visibility::Hidden`] roots that do not fit stay for a later frame.
pub fn drain_lod_cull(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut cull_writer: MessageWriter<LodCullRequest>,
	mut culling: Query<(Entity, &mut LodCullInFlight, Option<&mut LodChunkFulfillment>)>,
	children_q: Query<&Children>,
	hosts: Query<(), With<LodSceneHost>>,
	bags: Query<(), With<LodLevelRoots>>,
	level_roots: Query<(), With<LodLevelRoot>>,
	wants_cull: Query<(), With<LodCullInFlight>>,
) {
	let mut root_despawns = budget.cull_root_despawns_per_frame;
	if clock.cull_remaining == 0 && root_despawns == 0 {
		return;
	}

	let targets: Vec<Entity> = culling.iter().map(|(e, _, _)| e).collect();

	for entity in targets {
		let Ok((_, mut cull, fulfillment)) = culling.get_mut(entity) else {
			continue;
		};

		let nested_hosts = shallow_nested_hosts(entity, &children_q, &hosts);
		if !nested_hosts.is_empty() {
			for host in nested_hosts {
				enqueue_lod_cull(&mut commands, &mut cull_writer, host, &wants_cull);
			}
			continue;
		}

		if hosts.contains(entity) {
			let roots = bag_level_roots(entity, &children_q, &bags, &level_roots);
			if !roots.is_empty() {
				for root in roots {
					enqueue_lod_cull(&mut commands, &mut cull_writer, root, &wants_cull);
				}
				continue;
			}
		}

		let spawned = fulfillment.as_ref().map(|f| f.spawned).unwrap_or(0);
		if !cull.started {
			cull.started = true;
			if let Some(mut fulfillment) = fulfillment {
				fulfillment.queue.clear();
			}
		}

		let child_count = children_q.get(entity).map(|c| c.len()).unwrap_or(0);
		let weight = (spawned.max(child_count) as u32).max(DEFAULT_CHUNK_WEIGHT).max(1);
		let fits_weight = weight <= clock.cull_remaining;
		if !fits_weight && root_despawns == 0 {
			continue;
		}

		commands
			.entity(entity)
			.remove::<LodChunkFulfillment>()
			.remove::<LodLevelRootPending>();
		commands.entity(entity).despawn();
		if fits_weight {
			clock.cull_remaining = clock.cull_remaining.saturating_sub(weight);
		} else {
			root_despawns -= 1;
		}
	}
}
