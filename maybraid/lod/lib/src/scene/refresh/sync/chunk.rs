//! Incremental LOD level-root fulfillment via [`crate::SceneChunk`].
//!
//! Default sync path (vs optional eager [`super::eager::fulfill_lod_level_spawn`]):
//! builds a pending root, drains weighted primitives under
//! [`LodChunkFulfillBudget`], marks content [`LodLevelRootStreamed`], then
//! completes when nested [`LodSceneHost`]s are [`LodSceneHostStreamed`].
//!
//! Visibility policy:
//! - **Cold** (no ready level root yet): show the pending desired root while
//!   chunks stream in.
//! - **Warm** (a ready root already exists): keep that root visible until the
//!   incoming root is streamed and nested hosts are present (Streamed), then
//!   swap.
//!
//! Pipeline (within [`crate::LodRefreshSystems::Fulfill`]):
//! reset budget → cancel/sticky → begin jobs → drain spawn → complete / swap.
//! Teardown uses the same budget via [`super::cull::drain_lod_cull`] in Cull.
//!
//! Command / archetype apply cost is measured via Bevy's `system_commands`
//! tracing spans (requires the `trace` feature) at auto-[`ApplyDeferred`] points.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::Instant;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::scene::host::{
	LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use crate::lod_ref::{point_bounds, LodNode, LodNodeBounds, LodNodePose};

use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodRefreshSystems};
use super::cull::{
	apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, enqueue_lod_cull, LodCullEntity,
};

/// Marker: this [`LodLevelRoot`] is still awaiting warm-swap completion.
///
/// Content may already be [`LodLevelRootStreamed`] while nested hosts catch up.
/// Cold-start roots may be visible while pending.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootPending;

/// This level root's chunk plan is fully spawned (full scene representation).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootStreamed;

/// This [`LodSceneHost`] has a full scene representation available (Streamed).
///
/// Means at least one level root finished content streaming and nested hosts
/// under that root were Streamed. Does **not** require the host to be at its
/// current desired [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHostStreamed;

/// Entity is tearing down under [`super::cull::drain_lod_cull`].
///
/// Frozen fulfill plans stay until [`Self::started`] so sticky desired-level
/// resume can continue the same job. Once teardown spends budget, the plan is
/// dropped and sticky no longer applies.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodWantsCull {
	/// True after the first teardown step (plan cleared / child despawned).
	pub started: bool,
}

/// Remaining weighted primitives for a pending level root.
///
/// Frozen at [`begin_chunk_lod_fulfill`]: host mutability does not rewrite this
/// queue mid-job.
#[derive(Component)]
pub struct LodChunkFulfillment {
	pub queue: VecDeque<(u32, Box<dyn bevy::scene::Scene>)>,
	/// Primitive count at job begin (Streamed when `spawned == expected`).
	pub expected: usize,
	pub spawned: usize,
}

impl LodChunkFulfillment {
	fn is_content_complete(&self) -> bool {
		self.queue.is_empty() && self.spawned >= self.expected
	}
}

/// Per-frame weight budget for spawn **and** cull drains.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LodChunkFulfillBudget {
	/// Relative weight units drained across all jobs each frame.
	pub weights_per_frame: u32,
}

impl Default for LodChunkFulfillBudget {
	fn default() -> Self {
		Self {
			weights_per_frame: 512,
		}
	}
}

/// Remaining weight for the current frame (spawn drain then cull drain).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkBudgetClock {
	pub remaining: u32,
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

/// Reset [`LodChunkBudgetClock`] from [`LodChunkFulfillBudget`] each frame.
pub fn reset_lod_chunk_budget(
	budget: Res<LodChunkFulfillBudget>,
	mut clock: ResMut<LodChunkBudgetClock>,
) {
	clock.remaining = budget.weights_per_frame;
}

fn roots_bag_entity(
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

fn has_ready_root(
	root_children: &Children,
	root_keys: &Query<&LodLevelRoot>,
	pending: &Query<(), With<LodLevelRootPending>>,
	wants_cull: &Query<(), With<LodWantsCull>>,
) -> bool {
	for child in root_children.iter() {
		if root_keys.get(child).is_err() {
			continue;
		}
		if wants_cull.contains(child) {
			continue;
		}
		if !pending.contains(child) {
			return true;
		}
	}
	false
}

/// Depth-first collect of nested [`LodSceneHost`] entities under `root`.
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

fn nested_hosts_streamed(
	root: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
) -> bool {
	let mut nested = Vec::new();
	collect_nested_hosts(root, children_q, hosts, &mut nested);
	nested.iter().all(|host| streamed_hosts.contains(*host))
}

/// Enqueue cull for pending roots whose level is no longer desired; sticky-resume
/// desired pending roots that have not started teardown (keeps frozen plan).
pub fn cancel_stale_chunk_fulfillments(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullEntity>,
	hosts: Query<(Entity, &LodSceneLevel, Option<&Children>), With<LodSceneHost>>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	pending_roots: Query<&LodLevelRoot, With<LodLevelRootPending>>,
	wants_cull: Query<&LodWantsCull>,
	wants_cull_marker: Query<(), With<LodWantsCull>>,
) {
	let t0 = Instant::now();
	let mut enqueued = 0u32;
	let mut resumed = 0u32;
	for (_host, desired, host_children) in &hosts {
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
			if root.0 == *desired {
				if let Ok(cull) = wants_cull.get(child) {
					if !cull.started {
						commands.entity(child).remove::<LodWantsCull>();
						resumed += 1;
					}
				}
				continue;
			}
			if wants_cull_marker.contains(child) {
				continue;
			}
			enqueue_lod_cull(
				&mut commands,
				&mut cull_writer,
				child,
				&wants_cull_marker,
			);
			enqueued += 1;
		}
	}
	if enqueued > 0 || resumed > 0 {
		info!(
			"[lod.chunk] cancel_stale: enqueued={enqueued} sticky_resumed={resumed} in {:.2}ms",
			ms(t0)
		);
	}
}

/// Start a pending root + queue from [`LodLevelSpawnRequest`].
///
/// Cold start (no ready root): pending root is visible so chunks stream on-screen.
/// Warm switch: pending root stays Hidden until [`complete_chunk_lod_fulfill`].
///
/// Roots with [`LodWantsCull`] count as absent (dying); a new job may begin for
/// the same level once sticky resume is no longer possible.
pub fn begin_chunk_lod_fulfill<T: Component + LodScene>(
	mut commands: Commands,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
	mut diag: ResMut<LodChunkFulfillDiag>,
	hosts: Query<(Entity, &T, &LodLevelSpawnRequest, &Children), With<LodSceneHost>>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
	wants_cull: Query<(), With<LodWantsCull>>,
) {
	let Ok((viewer_entity, pose, viewer_bounds)) = viewer.single() else {
		return;
	};
	let driver_bounds = viewer_bounds
		.map(|b| b.0)
		.unwrap_or_else(|| point_bounds(pose.current.translation));
	let lod_ref = pose.as_lod_ref(viewer_entity, &driver_bounds);

	let t_sys = Instant::now();
	let mut jobs_started = 0u32;
	let mut chunks_ms_total = 0.0f64;
	let mut spawn_ms_total = 0.0f64;

	for (host, scene, request, host_children) in &hosts {
		let Some(roots_entity) = roots_bag_entity(host_children, &level_roots_heads) else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};

		let mut cold = true;
		if let Ok((_, Some(root_children))) = level_roots_heads.get(roots_entity) {
			let mut already_ready = false;
			let mut already_pending = false;
			for child in root_children.iter() {
				let Ok(root) = root_keys.get(child) else {
					continue;
				};
				if wants_cull.contains(child) {
					continue;
				}
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
			cold = !has_ready_root(root_children, &root_keys, &pending, &wants_cull);
		}

		let t_chunks = Instant::now();
		let chunk = scene.scene_chunks_with_level(&lod_ref, request.level);
		let queue = chunk.into_primitives();
		let chunks_ms = ms(t_chunks);
		chunks_ms_total += chunks_ms;
		let expected = queue.len();
		let queue_weight: u32 = queue.iter().map(|(w, _)| *w).sum();
		diag.last_scene_chunks_ms = chunks_ms;
		diag.last_level = Some(request.level);

		let t_spawn = Instant::now();
		let level = request.level;
		let initial_vis = if cold {
			Visibility::Inherited
		} else {
			Visibility::Hidden
		};
		let level_root = bsn! {
			template_value(LodLevelRoot(level))
			LodLevelRootPending
			Transform::default()
			template_value(initial_vis)
		};
		let root_entity = commands.spawn_scene(level_root).id();
		let fulfillment = LodChunkFulfillment {
			queue,
			expected,
			spawned: 0,
		};
		if fulfillment.is_content_complete() {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		commands.entity(root_entity).insert(fulfillment);
		commands.entity(roots_entity).add_child(root_entity);
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		let spawn_ms = ms(t_spawn);
		spawn_ms_total += spawn_ms;
		jobs_started += 1;

		info!(
			"[lod.chunk] begin job host={host:?} level={level:?} cold={cold}: \
			 scene_chunks_with_level={chunks_ms:.2}ms expected={expected} \
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

/// Drain weighted primitives under [`LodChunkBudgetClock`].
///
/// Always spawns at least one primitive per active job when the queue is
/// non-empty, even if that primitive's weight exceeds the remaining budget.
/// Marks [`LodLevelRootStreamed`] when `spawned == expected`.
/// Skips roots with [`LodWantsCull`].
pub fn drain_chunk_lod_fulfill(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut jobs: Query<
		(Entity, &mut LodChunkFulfillment),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
	>,
) {
	let t0 = Instant::now();
	let mut remaining = clock.remaining;
	let mut spawned = 0u32;
	let mut weight_spent = 0u32;
	let mut active_jobs = 0u32;
	let mut newly_streamed = 0u32;

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
			job.spawned += 1;
			spawned_this_job = true;
		}
		if job.is_content_complete() {
			commands.entity(root).insert(LodLevelRootStreamed);
			newly_streamed += 1;
		}
	}

	clock.remaining = remaining;

	if spawned > 0 || newly_streamed > 0 {
		info!(
			"[lod.chunk] drain: queued_spawns={spawned} weight_spent={weight_spent} \
			 budget={} active_jobs={active_jobs} newly_streamed={newly_streamed} \
			 queue_cmds={:.2}ms (apply cost: watch [lod.commands] system_commands)",
			budget.weights_per_frame,
			ms(t0)
		);
	}
}

/// Finish pending roots that are content-[`LodLevelRootStreamed`] and whose nested
/// hosts are [`LodSceneHostStreamed`]: clear pending, show root, hide siblings.
///
/// Warm switches wait here so parents do not reveal empty shells. Cold starts
/// may already be visible; this still clears pending once nested hosts present.
pub fn complete_chunk_lod_fulfill(
	mut commands: Commands,
	pending: Query<
		(
			Entity,
			Option<&LodChunkFulfillment>,
			Option<&ChildOf>,
			Has<LodLevelRootStreamed>,
		),
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
		if fulfillment.is_some_and(|f| !f.is_content_complete()) {
			continue;
		}
		if !content_streamed {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		if !nested_hosts_streamed(root_entity, &children_q, &nested_hosts, &streamed_hosts) {
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
			// Only hide other roots (ready or pending); keep structure intact.
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

/// Register incremental chunk fulfill systems for one [`LodScene`] host type.
pub fn add_lod_refresh_chunk_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
		app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
	}
}

/// Chunk fulfill plugin (default sync path for level-root spawn).
pub struct LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

/// Substeps within [`LodRefreshSystems::Cull`] (order against these, not system types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodChunkCullSystems {
	/// Mark unwanted roots / hosts ([`cull_lod_level_roots`], cancel already ran in Fulfill).
	Enqueue,
	/// Apply [`LodCullEntity`] → [`LodWantsCull`].
	Apply,
	/// Budgeted leaf-first despawn.
	Drain,
}

/// Shared chunk budget clock, cull messages, and one-shot drain registration.
pub struct LodChunkBudgetPlugin;

impl Plugin for LodChunkBudgetPlugin {
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<LodChunkFulfillBudget>()
			.init_resource::<LodChunkBudgetClock>()
			.init_resource::<LodChunkFulfillDiag>()
			.add_message::<LodCullEntity>()
			.configure_sets(
				Update,
				(
					LodChunkCullSystems::Enqueue,
					LodChunkCullSystems::Apply,
					LodChunkCullSystems::Drain,
				)
					.chain()
					.in_set(LodRefreshSystems::Cull),
			)
			.add_systems(
				Update,
				(
					reset_lod_chunk_budget.in_set(LodRefreshSystems::Fulfill),
					apply_lod_cull_requests.in_set(LodChunkCullSystems::Apply),
					drain_lod_cull.in_set(LodChunkCullSystems::Drain),
				),
			);
	}
}

fn ensure_chunk_budget(app: &mut App) {
	if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
		app.add_plugins(LodChunkBudgetPlugin);
	}
}

impl<T> Plugin for LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		// Cancel inserts [`LodWantsCull`] directly (no second `apply_lod_cull_requests`
		// here — duplicate SystemTypeSets break `.before(apply_…)` ordering).
		app.add_systems(
			Update,
			(
				cancel_stale_chunk_fulfillments,
				begin_chunk_lod_fulfill::<T>,
				drain_chunk_lod_fulfill,
				complete_chunk_lod_fulfill,
			)
				.chain()
				.in_set(LodRefreshSystems::Fulfill)
				.after(reset_lod_chunk_budget),
		);
	}
}

/// Probe-style levels + chunk fulfill + cull (no region message pipeline).
pub fn add_lod_refresh_chunk_full_for<T: Component + LodScene>(app: &mut App) {
	ensure_chunk_budget(app);
	app.add_systems(
		Update,
		(
			crate::scene::refresh::update_lod_host_levels::<T, (), With<LodViewer>>
				.in_set(LodRefreshSystems::UpdateLevels),
			cull_lod_level_roots::<T, (), With<LodViewer>>.in_set(LodChunkCullSystems::Enqueue),
		),
	);
	add_lod_refresh_chunk_for::<T>(app);
}

/// Default sync for message-driven refresh: chunk fulfill + cull.
pub struct LodSceneRefreshSyncPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, F> Plugin for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
		}
		app.add_systems(
			Update,
			cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
		);
	}
}
