//! Global weighted drain with Presence / Level priority and round-robin fairness.
//!
//! Only **desired** pending roots receive spawn budget (`root.level == host level`).
//! Not-desired jobs keep their [`LodChunkFulfillment`] queue (paused) until desired
//! again or a real [`super::super::cull::LodCullRequest`] tears them down.

use std::time::Instant;

use bevy::prelude::*;
use bevy::scene::prelude::bsn;

use crate::scene::host::{parent_host_desired_or_high, LodLevelRoot, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::schedule::{for_each_rr, split_presence_level, LevelBand};
use super::types::{
	LodChunkBandCursors, LodChunkBudgetClock, LodChunkDrainCursor, LodChunkFulfillBudget,
	LodChunkFulfillment, LodCullInFlight, LodLevelRootPending, LodLevelRootStreamed,
	LOD_CHUNK_TUPLE_BAND_COUNT,
};
use super::util::{host_desired_for_root, host_entity_for_root, ms};

struct TupleBuckets {
	bands: [Vec<Entity>; LOD_CHUNK_TUPLE_BAND_COUNT],
}

impl Default for TupleBuckets {
	fn default() -> Self {
		Self {
			bands: std::array::from_fn(|_| Vec::new()),
		}
	}
}

impl TupleBuckets {
	fn push(&mut self, entity: Entity, parent: LodSceneLevel, self_level: LodSceneLevel) {
		let rank = LevelBand::tuple_rank(
			LevelBand::from_level(parent),
			LevelBand::from_level(self_level),
		);
		self.bands[rank].push(entity);
	}
}

#[derive(Default)]
struct JobBuckets {
	presence: TupleBuckets,
	level: TupleBuckets,
}

impl JobBuckets {
	fn push(
		&mut self,
		entity: Entity,
		cold: bool,
		parent: LodSceneLevel,
		self_level: LodSceneLevel,
	) {
		if cold {
			self.presence.push(entity, parent, self_level);
		} else {
			self.level.push(entity, parent, self_level);
		}
	}
}

struct DrainStats {
	spawned: u32,
	weight_spent: u32,
	newly_streamed: u32,
	jobs_touched: u32,
	paused_skipped: u32,
}

type DrainJobs<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static LodLevelRoot, &'static mut LodChunkFulfillment),
	(With<LodLevelRootPending>, Without<LodCullInFlight>),
>;

/// Drain weighted primitives under [`LodChunkBudgetClock`].
///
/// Budget is split ~⅛ Presence (cold jobs) / ~⅞ Level (warm). Frame parity chooses
/// which class runs first; leftovers roll into the second class. Within each class,
/// jobs drain by `(parent_desired, self_level)` High→… (missing parent = High; RR
/// inside each tuple band).
///
/// Not-desired pending roots are skipped (paused); their queues are left intact.
pub fn drain_chunk_lod_fulfill(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut cursor: ResMut<LodChunkDrainCursor>,
	mut jobs: DrainJobs,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
) {
	let t0 = Instant::now();
	let total = clock.spawn_remaining;
	if total == 0 {
		return;
	}

	let mut buckets = JobBuckets::default();
	let mut paused_skipped = 0u32;
	for (entity, root, job) in &jobs {
		if job.queue.is_empty() {
			continue;
		}
		let Some(desired) = host_desired_for_root(entity, &child_of, &host_levels) else {
			paused_skipped += 1;
			continue;
		};
		if root.0 != desired {
			paused_skipped += 1;
			continue;
		}
		let Some(host) = host_entity_for_root(entity, &child_of) else {
			paused_skipped += 1;
			continue;
		};
		let parent = parent_host_desired_or_high(host, &child_of, &host_levels);
		buckets.push(entity, job.cold, parent, root.0);
	}

	let (presence_share, level_share) = split_presence_level(total);
	let presence_first = cursor.frame % 2 == 0;
	let mut stats = DrainStats {
		spawned: 0,
		weight_spent: 0,
		newly_streamed: 0,
		jobs_touched: 0,
		paused_skipped,
	};

	let mut remaining = if presence_first {
		presence_share
	} else {
		level_share
	};

	if presence_first {
		drain_tuple_bands(
			&mut commands,
			&mut jobs,
			&buckets.presence,
			&mut cursor.presence,
			&mut remaining,
			&mut stats,
		);
		remaining = remaining.saturating_add(level_share);
		drain_tuple_bands(
			&mut commands,
			&mut jobs,
			&buckets.level,
			&mut cursor.level,
			&mut remaining,
			&mut stats,
		);
	} else {
		drain_tuple_bands(
			&mut commands,
			&mut jobs,
			&buckets.level,
			&mut cursor.level,
			&mut remaining,
			&mut stats,
		);
		remaining = remaining.saturating_add(presence_share);
		drain_tuple_bands(
			&mut commands,
			&mut jobs,
			&buckets.presence,
			&mut cursor.presence,
			&mut remaining,
			&mut stats,
		);
	}

	clock.spawn_remaining = remaining;

	if stats.spawned > 0 || stats.newly_streamed > 0 {
		info!(
			"[lod.chunk] drain: queued_spawns={} weight_spent={} budget={} \
			 jobs_touched={} newly_streamed={} paused_skipped={} presence_first={presence_first} \
			 queue_cmds={:.2}ms (rest={remaining})",
			stats.spawned,
			stats.weight_spent,
			budget.spawn_weights_per_frame,
			stats.jobs_touched,
			stats.newly_streamed,
			stats.paused_skipped,
			ms(t0),
		);
	}
}

fn drain_tuple_bands(
	commands: &mut Commands,
	jobs: &mut DrainJobs,
	buckets: &TupleBuckets,
	cursors: &mut LodChunkBandCursors,
	remaining: &mut u32,
	stats: &mut DrainStats,
) {
	for rank in 0..LOD_CHUNK_TUPLE_BAND_COUNT {
		if *remaining == 0 {
			return;
		}
		for_each_rr(&buckets.bands[rank], &mut cursors.bands[rank], |&entity| {
			drain_one(commands, jobs, entity, remaining, stats)
		});
	}
}

/// Drain one job while budget remains. Returns `false` when the frame budget is empty.
fn drain_one(
	commands: &mut Commands,
	jobs: &mut DrainJobs,
	entity: Entity,
	remaining: &mut u32,
	stats: &mut DrainStats,
) -> bool {
	if *remaining == 0 {
		return false;
	}
	let Ok((_, _, mut job)) = jobs.get_mut(entity) else {
		return true;
	};
	if job.queue.is_empty() {
		return true;
	}
	stats.jobs_touched += 1;
	while !job.queue.is_empty() {
		if *remaining == 0 {
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
		commands.entity(entity).add_child(child);
		let w = weight.max(1);
		*remaining = remaining.saturating_sub(w);
		stats.weight_spent += w;
		stats.spawned += 1;
		job.spawned += 1;
	}
	if job.is_content_complete() {
		commands.entity(entity).insert(LodLevelRootStreamed);
		stats.newly_streamed += 1;
	}
	*remaining > 0
}
