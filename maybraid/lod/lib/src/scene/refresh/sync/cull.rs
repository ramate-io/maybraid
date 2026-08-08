//! Budgeted level-root / host teardown per [`LodScene::scene_lod_culls`].
//!
//! Unwanted roots are enqueued via [`LodCullEntity`] + [`LodWantsCull`] (not
//! hard-despawned). [`drain_lod_cull`] tears down leaf-first under the shared
//! [`super::chunk::LodChunkBudgetClock`]: nested [`LodSceneHost`]s must finish
//! before a parent despawns chunks, then itself.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
};
use crate::scene::chunk::DEFAULT_CHUNK_WEIGHT;
use crate::scene::cull::LodSceneCulls;
use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::super::entities::dominant_lod_ref;
use super::chunk::{
	LodChunkBudgetClock, LodChunkFulfillment, LodLevelRootPending, LodWantsCull,
};

/// Request that `entity` enter budgeted cull ([`LodWantsCull`]).
///
/// Applied by [`apply_lod_cull_requests`]. Cancel / cull helpers may also insert
/// [`LodWantsCull`] directly so the next chained system sees it after
/// `ApplyDeferred`.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodCullEntity {
	pub entity: Entity,
}

/// Insert [`LodWantsCull`] from [`LodCullEntity`] messages (idempotent).
pub fn apply_lod_cull_requests(
	mut commands: Commands,
	mut reader: MessageReader<LodCullEntity>,
	existing: Query<(), With<LodWantsCull>>,
) {
	for LodCullEntity { entity } in reader.read() {
		if existing.contains(*entity) {
			continue;
		}
		if let Ok(mut entity_commands) = commands.get_entity(*entity) {
			entity_commands.insert((LodWantsCull { started: false }, Visibility::Hidden));
		}
	}
}

/// Enqueue budgeted cull: message + component (hidden).
pub fn enqueue_lod_cull(
	commands: &mut Commands,
	writer: &mut MessageWriter<LodCullEntity>,
	entity: Entity,
	already: &Query<(), With<LodWantsCull>>,
) {
	if already.contains(entity) {
		return;
	}
	writer.write(LodCullEntity { entity });
	commands
		.entity(entity)
		.insert((LodWantsCull { started: false }, Visibility::Hidden));
}

/// Mark inactive [`LodLevelRoot`]s for budgeted cull per [`LodScene::scene_lod_culls`].
///
/// Never targets the host's current [`LodSceneLevel`]. Hidden roots not listed
/// stay warm for cheap band flips. Skips the host while any **active** (not
/// already culling) level root is still [`LodLevelRootPending`] so warm-hold
/// roots are not GC'd mid-swap.
pub fn cull_lod_level_roots<T, FHost, FNode>(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullEntity>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, FNode)>,
	hosts: Query<(&T, &LodSceneLevel, &Children), (With<LodSceneHost>, FHost)>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
	wants_cull: Query<(), With<LodWantsCull>>,
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

	let t0 = std::time::Instant::now();
	let mut enqueued = 0u32;

	for (scene, current, host_children) in &hosts {
		let Some(lod_ref) = dominant_lod_ref(scene, &refs) else {
			continue;
		};
		let culls = scene.scene_lod_culls(lod_ref, *current);
		if matches!(culls, LodSceneCulls::None) {
			continue;
		}

		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}
		let Some(roots_entity) = roots_entity else {
			continue;
		};
		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		// Active warm-swap pending (not already tearing down).
		if root_children.iter().any(|child| {
			pending.contains(child) && !wants_cull.contains(child)
		}) {
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
				enqueued += 1;
			}
		}
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if enqueued > 0 || elapsed_ms >= 0.5 {
		info!(
			"[lod.refresh] cull_lod_level_roots: enqueued={enqueued} in {elapsed_ms:.2}ms"
		);
	}
}

fn collect_nested_hosts(
	root: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	out: &mut Vec<Entity>,
) {
	let Ok(children) = children_q.get(root) else {
		return;
	};
	for child in children.iter() {
		if hosts.contains(child) {
			out.push(child);
		}
		collect_nested_hosts(child, children_q, hosts, out);
	}
}

fn collect_level_roots(
	root: Entity,
	children_q: &Query<&Children>,
	level_roots: &Query<(), With<LodLevelRoot>>,
	out: &mut Vec<Entity>,
) {
	let Ok(children) = children_q.get(root) else {
		return;
	};
	for child in children.iter() {
		if level_roots.contains(child) {
			out.push(child);
		}
		collect_level_roots(child, children_q, level_roots, out);
	}
}

/// Budgeted leaf-first teardown for [`LodWantsCull`] entities.
///
/// - If the subtree still has nested [`LodSceneHost`]s, enqueue those hosts and wait.
/// - [`LodSceneHost`]: enqueue all nested [`LodLevelRoot`]s; when none remain, despawn
///   remaining children then self.
/// - Otherwise: drop any frozen fulfill plan once teardown starts, despawn direct
///   children under [`LodChunkBudgetClock`] (weight [`DEFAULT_CHUNK_WEIGHT`] each),
///   then despawn self.
fn entity_depth(entity: Entity, child_of: &Query<&ChildOf>) -> u32 {
	let mut depth = 0u32;
	let mut current = entity;
	while let Ok(parent) = child_of.get(current) {
		depth = depth.saturating_add(1);
		current = parent.parent();
	}
	depth
}

pub fn drain_lod_cull(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	mut cull_writer: MessageWriter<LodCullEntity>,
	mut culling: Query<(Entity, &mut LodWantsCull, Option<&mut LodChunkFulfillment>)>,
	children_q: Query<&Children>,
	child_of: Query<&ChildOf>,
	hosts: Query<(), With<LodSceneHost>>,
	level_roots: Query<(), With<LodLevelRoot>>,
	wants_cull: Query<(), With<LodWantsCull>>,
) {
	if clock.remaining == 0 {
		return;
	}

	let t0 = std::time::Instant::now();
	let mut despawned = 0u32;
	let mut weight_spent = 0u32;
	let mut waiting_nested = 0u32;
	let mut targets: Vec<(Entity, u32)> = culling
		.iter()
		.map(|(e, _, _)| (e, entity_depth(e, &child_of)))
		.collect();
	// Deeper first so nested hosts / roots run before parents in one pass.
	targets.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));

	for (entity, _) in targets {
		if clock.remaining == 0 {
			break;
		}
		let Ok((_, mut cull, fulfillment)) = culling.get_mut(entity) else {
			continue;
		};

		let mut nested_hosts = Vec::new();
		collect_nested_hosts(entity, &children_q, &hosts, &mut nested_hosts);
		// Hosts under this entity (not counting self).
		nested_hosts.retain(|h| *h != entity);

		if !nested_hosts.is_empty() {
			for host in nested_hosts {
				enqueue_lod_cull(&mut commands, &mut cull_writer, host, &wants_cull);
			}
			waiting_nested += 1;
			continue;
		}

		if hosts.contains(entity) {
			let mut roots = Vec::new();
			collect_level_roots(entity, &children_q, &level_roots, &mut roots);
			if !roots.is_empty() {
				for root in roots {
					enqueue_lod_cull(&mut commands, &mut cull_writer, root, &wants_cull);
				}
				waiting_nested += 1;
				continue;
			}
		}

		if !cull.started {
			cull.started = true;
			if let Some(mut fulfillment) = fulfillment {
				fulfillment.queue.clear();
			}
			commands
				.entity(entity)
				.remove::<LodChunkFulfillment>()
				.remove::<LodLevelRootPending>();
		}

		let child_ids: Vec<Entity> = children_q
			.get(entity)
			.map(|c| c.iter().collect())
			.unwrap_or_default();

		if child_ids.is_empty() {
			let w = DEFAULT_CHUNK_WEIGHT.max(1);
			commands.entity(entity).despawn();
			clock.remaining = clock.remaining.saturating_sub(w);
			weight_spent += w;
			despawned += 1;
			continue;
		}

		let mut despawned_this = false;
		for child in child_ids {
			if clock.remaining == 0 && despawned_this {
				break;
			}
			let w = DEFAULT_CHUNK_WEIGHT.max(1);
			commands.entity(child).despawn();
			clock.remaining = clock.remaining.saturating_sub(w);
			weight_spent += w;
			despawned += 1;
			despawned_this = true;
		}
	}

	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if despawned > 0 || waiting_nested > 0 {
		info!(
			"[lod.chunk] drain_cull: despawned={despawned} weight_spent={weight_spent} \
			 waiting_nested={waiting_nested} budget_left={} queue_cmds={elapsed_ms:.2}ms \
			 (apply cost: watch [lod.commands] system_commands)",
			clock.remaining
		);
	}
}
