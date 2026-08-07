//! Incremental LOD level-root fulfillment via [`crate::SceneChunk`].
//!
//! Parallel to eager [`crate::fulfill_lod_level_spawn`]: same Sync request, but
//! builds a hidden pending root, drains weighted primitives under
//! [`LodChunkFulfillBudget`], then atomically swaps visibility.
//!
//! Pipeline (within [`crate::LodFinePassSystems::Fulfill`]):
//! cancel stale → begin jobs → drain budget → complete / swap.
//!
//! Command / archetype apply cost is measured via Bevy's `system_commands`
//! tracing spans (requires the `trace` feature) at auto-[`ApplyDeferred`] points.

use std::collections::VecDeque;
use std::time::Instant;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::fine_pass::{ephemeral_bounds, LodFinePassSystems, LodHostBounds, LodViewerState};
use crate::gen::LodScene;
use crate::lod_level::LodSceneLevel;
use crate::lod_scene_host::{
	LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};

/// Marker: this [`LodLevelRoot`] is still receiving chunk primitives (keep Hidden).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootPending;

/// Remaining weighted primitives for a pending level root.
#[derive(Component)]
pub struct LodChunkFulfillment {
	pub queue: VecDeque<(u32, Box<dyn bevy::scene::Scene>)>,
}

/// Per-frame weight budget for draining all chunk fulfillments.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LodChunkFulfillBudget {
	/// Relative weight units drained across all jobs each frame.
	pub weights_per_frame: u32,
}

impl Default for LodChunkFulfillBudget {
	fn default() -> Self {
		Self { weights_per_frame: 4 }
	}
}

/// Diagnostic: last `scene_chunks_with_level` timing (scene build, not apply).
#[derive(Resource, Debug, Default)]
pub struct LodChunkFulfillDiag {
	pub last_scene_chunks_ms: f64,
	pub last_level: Option<LodSceneLevel>,
}

fn ms(start: Instant) -> f64 {
	start.elapsed().as_secs_f64() * 1000.0
}

/// Cancel pending roots whose level is no longer desired.
pub fn cancel_stale_chunk_fulfillments(
	mut commands: Commands,
	hosts: Query<(Entity, &LodSceneLevel, Option<&Children>), With<LodSceneHost>>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	pending_roots: Query<&LodLevelRoot, With<LodLevelRootPending>>,
) {
	let t0 = Instant::now();
	let mut despawned = 0u32;
	for (host, desired, host_children) in &hosts {
		let Some(host_children) = host_children else {
			continue;
		};
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
		for child in root_children.iter() {
			let Ok(root) = pending_roots.get(child) else {
				continue;
			};
			if root.0 != *desired {
				commands.entity(child).despawn();
				despawned += 1;
			}
		}
		let _ = host;
	}
	if despawned > 0 {
		info!(
			"[lod.chunk] cancel_stale: {despawned} despawned in {:.2}ms",
			ms(t0)
		);
	}
}

/// Start a hidden pending root + queue from [`LodLevelSpawnRequest`].
pub fn begin_chunk_lod_fulfill<T: Component + LodScene>(
	mut commands: Commands,
	viewer: Res<LodViewerState>,
	mut diag: ResMut<LodChunkFulfillDiag>,
	hosts: Query<
		(Entity, &T, Option<&LodHostBounds>, &LodLevelSpawnRequest, &Children),
		With<LodSceneHost>,
	>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}

	let t_sys = Instant::now();
	let mut jobs_started = 0u32;
	let mut chunks_ms_total = 0.0f64;
	let mut spawn_ms_total = 0.0f64;

	for (host, scene, host_bounds, request, host_children) in &hosts {
		let bounds = ephemeral_bounds(host_bounds);
		let lod_ref = viewer.lod_ref(&bounds);

		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}
		let Some(roots_entity) = roots_entity else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};

		if let Ok((_, Some(root_children))) = level_roots_heads.get(roots_entity) {
			let mut already_ready = false;
			let mut already_pending = false;
			for child in root_children.iter() {
				let Ok(root) = root_keys.get(child) else {
					continue;
				};
				if root.0 != request.level {
					continue;
				}
				if pending.contains(child) {
					already_pending = true;
				} else {
					already_ready = true;
				}
			}
			if already_ready || already_pending {
				commands.entity(host).remove::<LodLevelSpawnRequest>();
				continue;
			}
		}

		let t_chunks = Instant::now();
		let chunk = scene.scene_chunks_with_level(&lod_ref, request.level);
		let queue = chunk.into_primitives();
		let chunks_ms = ms(t_chunks);
		chunks_ms_total += chunks_ms;
		let queue_len = queue.len();
		let queue_weight: u32 = queue.iter().map(|(w, _)| *w).sum();
		diag.last_scene_chunks_ms = chunks_ms;
		diag.last_level = Some(request.level);

		let t_spawn = Instant::now();
		let level = request.level;
		let level_root = bsn! {
			template_value(LodLevelRoot(level))
			LodLevelRootPending
			Transform::default()
			Visibility::Hidden
		};
		let root_entity = commands.spawn_scene(level_root).id();
		commands.entity(root_entity).insert(LodChunkFulfillment { queue });
		commands.entity(roots_entity).add_child(root_entity);
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		let spawn_ms = ms(t_spawn);
		spawn_ms_total += spawn_ms;
		jobs_started += 1;

		info!(
			"[lod.chunk] begin job host={host:?} level={level:?}: \
			 scene_chunks_with_level={chunks_ms:.2}ms queue_len={queue_len} \
			 queue_weight={queue_weight} queue_root_cmds={spawn_ms:.2}ms"
		);
	}

	if jobs_started > 0 {
		info!(
			"[lod.chunk] begin_chunk_lod_fulfill: {jobs_started} jobs, \
			 scene_chunks_with_level={chunks_ms_total:.2}ms queue_root_cmds={spawn_ms_total:.2}ms \
			 total={:.2}ms",
			ms(t_sys)
		);
	}
}

/// Drain weighted primitives under [`LodChunkFulfillBudget`].
///
/// Always spawns at least one primitive per active job when the queue is
/// non-empty, even if that primitive's weight exceeds the remaining budget.
pub fn drain_chunk_lod_fulfill(
	mut commands: Commands,
	budget: Res<LodChunkFulfillBudget>,
	mut jobs: Query<(Entity, &mut LodChunkFulfillment), With<LodLevelRootPending>>,
) {
	let t0 = Instant::now();
	let mut remaining = budget.weights_per_frame;
	let mut spawned = 0u32;
	let mut weight_spent = 0u32;
	let mut active_jobs = 0u32;

	for (root, mut job) in &mut jobs {
		if job.queue.is_empty() {
			continue;
		}
		active_jobs += 1;
		let mut spawned_this_job = false;
		while !job.queue.is_empty() {
			if remaining == 0 && spawned_this_job {
				break;
			}
			let Some((weight, scene)) = job.queue.pop_front() else {
				break;
			};
			let children = vec![scene];
			let piece = bsn! {
				Transform::default()
				Visibility::Inherited
				Children [ {children} ]
			};
			let child = commands.spawn_scene(piece).id();
			commands.entity(root).add_child(child);
			let w = weight.max(1);
			remaining = remaining.saturating_sub(w);
			weight_spent += w;
			spawned += 1;
			spawned_this_job = true;
		}
	}

	if spawned > 0 {
		info!(
			"[lod.chunk] drain: queued_spawns={spawned} weight_spent={weight_spent} \
			 budget={} active_jobs={active_jobs} queue_cmds={:.2}ms \
			 (apply cost: watch [lod.commands] system_commands)",
			budget.weights_per_frame,
			ms(t0)
		);
	}
}

/// When a pending root's queue is empty: clear pending and show it; hide siblings.
pub fn complete_chunk_lod_fulfill(
	mut commands: Commands,
	pending: Query<
		(Entity, Option<&LodChunkFulfillment>, Option<&ChildOf>),
		With<LodLevelRootPending>,
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	mut visibilities: Query<&mut Visibility>,
) {
	let t0 = Instant::now();
	let mut completed = 0u32;
	for (root_entity, fulfillment, child_of) in &pending {
		if fulfillment.is_some_and(|f| !f.queue.is_empty()) {
			continue;
		}
		commands.entity(root_entity).remove::<LodChunkFulfillment>().remove::<LodLevelRootPending>();
		if let Ok(mut vis) = visibilities.get_mut(root_entity) {
			*vis = Visibility::Inherited;
		}
		completed += 1;

		let Some(child_of) = child_of else {
			continue;
		};
		let parent = child_of.0;
		let Ok(siblings) = level_roots_heads.get(parent) else {
			continue;
		};
		for sibling in siblings.iter() {
			if sibling == root_entity {
				continue;
			}
			if let Ok(mut vis) = visibilities.get_mut(sibling) {
				*vis = Visibility::Hidden;
			}
		}
	}
	if completed > 0 {
		info!(
			"[lod.chunk] complete: {completed} roots swapped in {:.2}ms",
			ms(t0)
		);
	}
}

/// Register incremental fulfill systems for one [`LodScene`] host type.
///
/// Does **not** register eager [`crate::fulfill_lod_level_spawn`]. Use instead of
/// (or in place of) the fulfill half of [`crate::add_fine_pass_for`].
pub fn add_fine_pass_chunk_for<T: Component + LodScene>(app: &mut App) {
	crate::fine_pass::configure_fine_pass_sets(app);
	app.init_resource::<LodChunkFulfillBudget>()
		.init_resource::<LodChunkFulfillDiag>()
		.add_systems(
			Update,
			(
				cancel_stale_chunk_fulfillments.in_set(LodFinePassSystems::Fulfill),
				begin_chunk_lod_fulfill::<T>.in_set(LodFinePassSystems::Fulfill),
				drain_chunk_lod_fulfill.in_set(LodFinePassSystems::Fulfill),
				complete_chunk_lod_fulfill.in_set(LodFinePassSystems::Fulfill),
			)
				// Auto-ApplyDeferred between steps applies Commands (measured via
				// `system_commands` spans when the `trace` feature is enabled).
				.chain(),
		);
}

/// Like [`crate::add_fine_pass_for`], but uses chunk fulfill instead of eager spawn.
pub fn add_fine_pass_chunk_full_for<T: Component + LodScene>(app: &mut App) {
	crate::fine_pass::configure_fine_pass_sets(app);
	app.add_systems(
		Update,
		(
			crate::fine_pass::update_lod_host_levels::<T>.in_set(LodFinePassSystems::UpdateLevels),
			crate::fine_pass::cull_lod_level_roots::<T>.in_set(LodFinePassSystems::Cull),
		),
	);
	add_fine_pass_chunk_for::<T>(app);
}
