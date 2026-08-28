//! Exclusive semantic drain: `World::spawn_scene` under weight **and** time.
//!
//! - **Presence**: cold jobs (`job.cold`, empty→something).
//! - **Desired**: warm jobs for the host's desired level root.
//! - **Active**: warm jobs on a **shown** (non-Hidden) root that is not desired —
//!   warm-hold continuation while an upgrade builds.
//!
//! Classification uses frozen `host` / `parent_desired` on [`LodChunkFulfillment`].
//! Budget ~¼ / ~⅜ / ~⅜ with leftovers cascading. Within each class, jobs drain by
//! `(parent_desired, self_level)` High→… (RR per tuple).
//!
//! `pull_primitive` + `World::spawn_scene` is one timed quantum. Stop **before**
//! the next pull when elapsed ≥ [`LodChunkFulfillBudget::spawn_time_per_frame`].
//! Warn if a quantum exceeds [`LodChunkFulfillBudget::max_atomic_spawn_cost`].

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::scene::prelude::WorldSceneExt;

use crate::scene::host::{lod_root_is_shown, LodLevelRoot, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::schedule::{class_order, for_each_rr, split_presence_desired_active, LevelBand};
use super::types::{
	FulfillClass, LodChunkAtomicOverrun, LodChunkBandCursors, LodChunkBudgetClock,
	LodChunkDrainCursor, LodChunkDrainDiagnostics, LodChunkFulfillBudget, LodChunkFulfillment,
	LodCullInFlight, LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed,
	LOD_CHUNK_TUPLE_BAND_COUNT,
};

/// Test-only: treat the exclusive clock as expired after this many pull+spawn quanta.
#[derive(Resource, Debug, Clone, Copy)]
pub(super) struct LodChunkDrainTimeScript {
	pub expire_after_quanta: u32,
}

struct TupleBuckets {
	bands: [Vec<Entity>; LOD_CHUNK_TUPLE_BAND_COUNT],
}

impl Default for TupleBuckets {
	fn default() -> Self {
		Self { bands: std::array::from_fn(|_| Vec::new()) }
	}
}

impl TupleBuckets {
	fn push(&mut self, entity: Entity, parent: LodSceneLevel, self_level: LodSceneLevel) {
		let rank =
			LevelBand::tuple_rank(LevelBand::from_level(parent), LevelBand::from_level(self_level));
		self.bands[rank].push(entity);
	}
}

#[derive(Default)]
struct JobBuckets {
	presence: TupleBuckets,
	desired: TupleBuckets,
	active: TupleBuckets,
}

impl JobBuckets {
	fn for_class_mut(&mut self, class: FulfillClass) -> &mut TupleBuckets {
		match class {
			FulfillClass::Presence => &mut self.presence,
			FulfillClass::Desired => &mut self.desired,
			FulfillClass::Active => &mut self.active,
		}
	}

	fn for_class(&self, class: FulfillClass) -> &TupleBuckets {
		match class {
			FulfillClass::Presence => &self.presence,
			FulfillClass::Desired => &self.desired,
			FulfillClass::Active => &self.active,
		}
	}
}

struct JobSnapshot {
	entity: Entity,
	self_level: LodSceneLevel,
	host: Entity,
	cold: bool,
	parent_desired: LodSceneLevel,
	queue_empty: bool,
}

struct DrainClock {
	start: Instant,
	quanta: u32,
}

impl DrainClock {
	fn new() -> Self {
		Self { start: Instant::now(), quanta: 0 }
	}

	fn time_up(&self, world: &World, budget: &LodChunkFulfillBudget) -> bool {
		if let Some(script) = world.get_resource::<LodChunkDrainTimeScript>() {
			return self.quanta >= script.expire_after_quanta;
		}
		self.start.elapsed() >= budget.spawn_time_per_frame
	}
}

/// Exclusive semantic drain. Times `Lazy::next` + `World::spawn_scene` as one quantum.
pub fn drain_chunk_lod_fulfill(world: &mut World) {
	let budget = *world.resource::<LodChunkFulfillBudget>();
	let total = world.resource::<LodChunkBudgetClock>().spawn_remaining;
	if total == 0 {
		world.resource_mut::<LodChunkDrainDiagnostics>().last_exclusive_elapsed = Duration::ZERO;
		return;
	}

	let mut clock = DrainClock::new();
	let snapshots = collect_job_snapshots(world);
	let mut buckets = JobBuckets::default();
	for snap in snapshots {
		if snap.queue_empty {
			continue;
		}
		let Some(desired) = world.get::<LodSceneLevel>(snap.host).copied() else {
			continue;
		};
		let shown = world.get::<Visibility>(snap.entity).is_some_and(|v| lod_root_is_shown(*v));
		let class = if snap.cold {
			if snap.self_level != desired {
				continue;
			}
			FulfillClass::Presence
		} else if snap.self_level == desired {
			FulfillClass::Desired
		} else if shown {
			FulfillClass::Active
		} else {
			continue;
		};
		buckets
			.for_class_mut(class)
			.push(snap.entity, snap.parent_desired, snap.self_level);
	}

	let (presence_share, desired_share, active_share) = split_presence_desired_active(total);
	let frame = world.resource::<LodChunkDrainCursor>().frame;
	let order = class_order(frame);
	let share = |c: FulfillClass| match c {
		FulfillClass::Presence => presence_share,
		FulfillClass::Desired => desired_share,
		FulfillClass::Active => active_share,
	};

	let mut remaining = 0u32;
	for class in order {
		remaining = remaining.saturating_add(share(class));
		let mut cursors = match class {
			FulfillClass::Presence => world.resource::<LodChunkDrainCursor>().presence,
			FulfillClass::Desired => world.resource::<LodChunkDrainCursor>().desired,
			FulfillClass::Active => world.resource::<LodChunkDrainCursor>().active,
		};
		drain_tuple_bands(
			world,
			buckets.for_class(class),
			&mut cursors,
			&mut remaining,
			&budget,
			&mut clock,
		);
		match class {
			FulfillClass::Presence => {
				world.resource_mut::<LodChunkDrainCursor>().presence = cursors
			}
			FulfillClass::Desired => world.resource_mut::<LodChunkDrainCursor>().desired = cursors,
			FulfillClass::Active => world.resource_mut::<LodChunkDrainCursor>().active = cursors,
		}
	}

	world.resource_mut::<LodChunkBudgetClock>().spawn_remaining = remaining;
	world.resource_mut::<LodChunkDrainDiagnostics>().last_exclusive_elapsed = clock.start.elapsed();
}

fn collect_job_snapshots(world: &mut World) -> Vec<JobSnapshot> {
	let mut jobs = world.query_filtered::<(Entity, &LodLevelRoot, &LodChunkFulfillment), (
		With<LodLevelRootPending>,
		Without<LodCullInFlight>,
	)>();
	jobs.iter(world)
		.map(|(entity, root, job)| JobSnapshot {
			entity,
			self_level: root.0,
			host: job.host,
			cold: job.cold,
			parent_desired: job.parent_desired,
			queue_empty: job.queue.is_empty(),
		})
		.collect()
}

fn drain_tuple_bands(
	world: &mut World,
	buckets: &TupleBuckets,
	cursors: &mut LodChunkBandCursors,
	remaining: &mut u32,
	budget: &LodChunkFulfillBudget,
	clock: &mut DrainClock,
) {
	for rank in 0..LOD_CHUNK_TUPLE_BAND_COUNT {
		if *remaining == 0 || clock.time_up(world, budget) {
			return;
		}
		for_each_rr(&buckets.bands[rank], &mut cursors.bands[rank], |&entity| {
			drain_one(world, entity, remaining, budget, clock)
		});
	}
}

fn drain_one(
	world: &mut World,
	entity: Entity,
	remaining: &mut u32,
	budget: &LodChunkFulfillBudget,
	clock: &mut DrainClock,
) -> bool {
	if *remaining == 0 || clock.time_up(world, budget) {
		return false;
	}
	if world.get::<LodChunkFulfillment>(entity).is_none() {
		return true;
	}
	if world.get::<LodChunkFulfillment>(entity).is_some_and(|j| j.queue.is_empty()) {
		return true;
	}

	loop {
		if *remaining == 0 || clock.time_up(world, budget) {
			break;
		}
		let quantum_start = Instant::now();
		let Some((weight, scene, host, level)) = pull_next(world, entity) else {
			break;
		};
		match world.spawn_scene(scene) {
			Ok(spawned) => {
				let child = spawned.id();
				world.entity_mut(entity).add_child(child);
			}
			Err(err) => {
				warn!(
					?err,
					host = ?host,
					level = ?level,
					"semantic LOD World::spawn_scene failed"
				);
			}
		}
		clock.quanta = clock.quanta.saturating_add(1);
		let quantum = quantum_start.elapsed();
		let over = budget.max_atomic_spawn_cost.is_zero() || quantum > budget.max_atomic_spawn_cost;
		if over {
			warn!(
				host = ?host,
				level = ?level,
				elapsed_us = quantum.as_micros(),
				max_us = budget.max_atomic_spawn_cost.as_micros(),
				"semantic LOD spawn quantum exceeded max_atomic_spawn_cost"
			);
			world
				.resource_mut::<LodChunkDrainDiagnostics>()
				.record_overrun(LodChunkAtomicOverrun { host, level, elapsed: quantum });
		}
		if let Some(mut job) = world.get_mut::<LodChunkFulfillment>(entity) {
			job.spawned += 1;
		}
		*remaining = remaining.saturating_sub(weight.max(1));
	}

	finish_if_complete(world, entity);
	*remaining > 0 && !clock.time_up(world, budget)
}

fn pull_next(
	world: &mut World,
	entity: Entity,
) -> Option<(u32, Box<dyn bevy::scene::prelude::Scene>, Entity, LodSceneLevel)> {
	let level = world.get::<LodLevelRoot>(entity)?.0;
	let mut job = world.get_mut::<LodChunkFulfillment>(entity)?;
	if job.queue.is_empty() {
		return None;
	}
	let host = job.host;
	let (weight, scene) = crate::scene::chunk::pull_primitive(&mut job.queue)?;
	Some((weight, scene, host, level))
}

fn finish_if_complete(world: &mut World, entity: Entity) {
	let Some(job) = world.get::<LodChunkFulfillment>(entity) else {
		return;
	};
	if !job.is_content_complete() {
		return;
	}
	let needs_nested = job.nested_required.is_none();
	world.entity_mut(entity).insert(LodLevelRootStreamed);
	if !needs_nested {
		return;
	}
	let (required, streamed) = count_nested_hosts_world(world, entity);
	if let Some(mut job) = world.get_mut::<LodChunkFulfillment>(entity) {
		job.nested_required = Some(required);
		job.nested_streamed = job.nested_streamed.max(streamed);
	}
}

fn count_nested_hosts_world(world: &World, root: Entity) -> (usize, usize) {
	let Some(children) = world.get::<Children>(root) else {
		return (0, 0);
	};
	let kids: Vec<Entity> = children.iter().collect();
	let mut required = 0usize;
	let mut streamed = 0usize;
	for child in kids {
		let Some(is_streamed) = next_level_host_streamed(world, child) else {
			continue;
		};
		required += 1;
		if is_streamed {
			streamed += 1;
		}
	}
	(required, streamed)
}

fn next_level_host_streamed(world: &World, entity: Entity) -> Option<bool> {
	if world.get::<LodSceneHost>(entity).is_some() {
		return Some(world.get::<LodSceneHostStreamed>(entity).is_some());
	}
	let kids: Vec<Entity> = world.get::<Children>(entity)?.iter().collect();
	for kid in kids {
		if world.get::<LodSceneHost>(kid).is_some() {
			return Some(world.get::<LodSceneHostStreamed>(kid).is_some());
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::scene::chunk::SceneChunk;
	use crate::scene::host::LodSceneHost;
	use crate::scene::refresh::sync::chunk::types::LodLevelRootPending;
	use bevy::asset::AssetPlugin;
	use bevy::scene::{ResolveContext, ResolvedScene, ScenePlugin};
	use std::collections::VecDeque;

	fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

	fn weighted_queue(n: usize) -> VecDeque<SceneChunk> {
		(0..n)
			.map(|_| SceneChunk::weighted(1, bevy::scene::SceneFunction(empty_scene)))
			.collect()
	}

	fn drain_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_resource::<LodChunkFulfillBudget>()
			.init_resource::<LodChunkBudgetClock>()
			.init_resource::<LodChunkDrainCursor>()
			.init_resource::<LodChunkDrainDiagnostics>();
		app
	}

	fn spawn_job(
		world: &mut World,
		desired: LodSceneLevel,
		root_level: LodSceneLevel,
		cold: bool,
		vis: Visibility,
		primitives: usize,
	) -> (Entity, Entity) {
		let host = world.spawn((LodSceneHost, desired)).id();
		let fulfillment = LodChunkFulfillment {
			queue: weighted_queue(primitives),
			expected: primitives,
			spawned: 0,
			cold,
			host,
			parent_desired: LodSceneLevel::High,
			nested_streamed: 0,
			nested_required: None,
		};
		let root = world
			.spawn((LodLevelRoot(root_level), LodLevelRootPending, vis, fulfillment))
			.id();
		(host, root)
	}

	#[test]
	fn stops_mid_job_when_time_budget_exhausted() {
		let mut app = drain_app();
		app.world_mut()
			.insert_resource(LodChunkDrainTimeScript { expire_after_quanta: 1 });
		app.world_mut().resource_mut::<LodChunkBudgetClock>().spawn_remaining = 64;
		let (_, root) = spawn_job(
			app.world_mut(),
			LodSceneLevel::High,
			LodSceneLevel::High,
			true,
			Visibility::Inherited,
			5,
		);

		drain_chunk_lod_fulfill(app.world_mut());

		let spawned = app.world().get::<LodChunkFulfillment>(root).unwrap().spawned;
		assert_eq!(spawned, 1, "time budget must stop before the second pull");
		assert!(app.world().get::<LodLevelRootStreamed>(root).is_none());
	}

	#[test]
	fn respects_class_shares_when_time_remains() {
		let mut app = drain_app();
		app.world_mut().resource_mut::<LodChunkFulfillBudget>().spawn_time_per_frame =
			Duration::from_secs(60);
		app.world_mut().resource_mut::<LodChunkBudgetClock>().spawn_remaining = 8;
		app.world_mut().resource_mut::<LodChunkDrainCursor>().frame = 0;

		let (_, presence) = spawn_job(
			app.world_mut(),
			LodSceneLevel::High,
			LodSceneLevel::High,
			true,
			Visibility::Inherited,
			10,
		);
		let (_, desired) = spawn_job(
			app.world_mut(),
			LodSceneLevel::High,
			LodSceneLevel::High,
			false,
			Visibility::Hidden,
			10,
		);
		let (_, active) = spawn_job(
			app.world_mut(),
			LodSceneLevel::High,
			LodSceneLevel::Medium,
			false,
			Visibility::Inherited,
			10,
		);

		drain_chunk_lod_fulfill(app.world_mut());

		assert_eq!(app.world().get::<LodChunkFulfillment>(presence).unwrap().spawned, 2);
		assert_eq!(app.world().get::<LodChunkFulfillment>(desired).unwrap().spawned, 3);
		assert_eq!(app.world().get::<LodChunkFulfillment>(active).unwrap().spawned, 3);
	}

	#[test]
	fn over_budget_quantum_records_host_and_level() {
		let mut app = drain_app();
		app.world_mut().resource_mut::<LodChunkFulfillBudget>().max_atomic_spawn_cost =
			Duration::ZERO;
		app.world_mut().resource_mut::<LodChunkFulfillBudget>().spawn_time_per_frame =
			Duration::from_secs(60);
		app.world_mut().resource_mut::<LodChunkBudgetClock>().spawn_remaining = 1;
		let (host, root) = spawn_job(
			app.world_mut(),
			LodSceneLevel::Low,
			LodSceneLevel::Low,
			true,
			Visibility::Inherited,
			1,
		);

		drain_chunk_lod_fulfill(app.world_mut());

		let diag = app.world().resource::<LodChunkDrainDiagnostics>();
		assert_eq!(diag.atomic_overruns.len(), 1);
		assert_eq!(diag.atomic_overruns[0].host, host);
		assert_eq!(diag.atomic_overruns[0].level, LodSceneLevel::Low);
		assert!(app.world().get::<LodLevelRootStreamed>(root).is_some());
	}
}
