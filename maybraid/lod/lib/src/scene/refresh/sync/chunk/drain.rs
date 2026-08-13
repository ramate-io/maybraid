//! Global weighted drain with Presence / Desired / Active priority.
//!
//! - **Presence**: cold jobs (`job.cold`, empty→something).
//! - **Desired**: warm jobs for the host's desired level root.
//! - **Active**: warm jobs on a **shown** (non-Hidden) root that is not desired —
//!   warm-hold continuation while an upgrade builds.
//!
//! Classification uses frozen `host` / `parent_desired` on [`LodChunkFulfillment`].
//! Budget ~¼ / ~⅜ / ~⅜ with leftovers cascading. Within each class, jobs drain by
//! `(parent_desired, self_level)` High→… (RR per tuple).

use std::time::Instant;

use bevy::prelude::*;
use bevy::scene::prelude::bsn;

use crate::lod_chunk_trace;
use crate::scene::host::{lod_root_is_shown, LodLevelRoot, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::schedule::{class_order, for_each_rr, split_presence_desired_active, LevelBand};
use super::types::{
	FulfillClass, LodChunkBandCursors, LodChunkBudgetClock, LodChunkDrainCursor,
	LodChunkFulfillBudget, LodChunkFulfillment, LodCullInFlight, LodLevelRootPending,
	LodLevelRootStreamed, LodSceneHostStreamed, LOD_CHUNK_TUPLE_BAND_COUNT,
};
use super::util::{count_nested_hosts, ms};

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
pub fn drain_chunk_lod_fulfill(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut cursor: ResMut<LodChunkDrainCursor>,
	mut jobs: DrainJobs,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	visibilities: Query<&Visibility>,
	children_q: Query<&Children>,
	nested_hosts: Query<(), With<LodSceneHost>>,
	streamed_hosts: Query<(), With<LodSceneHostStreamed>>,
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
		let Ok(desired) = host_levels.get(job.host) else {
			paused_skipped += 1;
			continue;
		};
		let shown = visibilities.get(entity).ok().is_some_and(|v| lod_root_is_shown(*v));
		let class = if job.cold {
			if root.0 != *desired {
				paused_skipped += 1;
				continue;
			}
			FulfillClass::Presence
		} else if root.0 == *desired {
			FulfillClass::Desired
		} else if shown {
			FulfillClass::Active
		} else {
			paused_skipped += 1;
			continue;
		};
		buckets.for_class_mut(class).push(entity, job.parent_desired, root.0);
	}

	let (presence_share, desired_share, active_share) = split_presence_desired_active(total);
	let order = class_order(cursor.frame);
	let mut stats = DrainStats {
		spawned: 0,
		weight_spent: 0,
		newly_streamed: 0,
		jobs_touched: 0,
		paused_skipped,
	};

	let share = |c: FulfillClass| match c {
		FulfillClass::Presence => presence_share,
		FulfillClass::Desired => desired_share,
		FulfillClass::Active => active_share,
	};

	let mut remaining = 0u32;
	for class in order {
		remaining = remaining.saturating_add(share(class));
		let cursors = match class {
			FulfillClass::Presence => &mut cursor.presence,
			FulfillClass::Desired => &mut cursor.desired,
			FulfillClass::Active => &mut cursor.active,
		};
		drain_tuple_bands(
			&mut commands,
			&mut jobs,
			buckets.for_class(class),
			cursors,
			&mut remaining,
			&mut stats,
			&children_q,
			&nested_hosts,
			&streamed_hosts,
		);
	}

	clock.spawn_remaining = remaining;

	if lod_chunk_trace() && (stats.spawned > 0 || stats.newly_streamed > 0) {
		info!(
			"[lod.chunk] drain: queued_spawns={} weight_spent={} budget={} \
			 jobs_touched={} newly_streamed={} paused_skipped={} first={:?} \
			 queue_cmds={:.2}ms (rest={remaining})",
			stats.spawned,
			stats.weight_spent,
			budget.spawn_weights_per_frame,
			stats.jobs_touched,
			stats.newly_streamed,
			stats.paused_skipped,
			order[0],
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
	children_q: &Query<&Children>,
	nested_hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
) {
	for rank in 0..LOD_CHUNK_TUPLE_BAND_COUNT {
		if *remaining == 0 {
			return;
		}
		for_each_rr(&buckets.bands[rank], &mut cursors.bands[rank], |&entity| {
			drain_one(
				commands,
				jobs,
				entity,
				remaining,
				stats,
				children_q,
				nested_hosts,
				streamed_hosts,
			)
		});
	}
}

fn drain_one(
	commands: &mut Commands,
	jobs: &mut DrainJobs,
	entity: Entity,
	remaining: &mut u32,
	stats: &mut DrainStats,
	children_q: &Query<&Children>,
	nested_hosts: &Query<(), With<LodSceneHost>>,
	streamed_hosts: &Query<(), With<LodSceneHostStreamed>>,
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
		if job.nested_required.is_none() {
			let (required, streamed) =
				count_nested_hosts(entity, children_q, nested_hosts, streamed_hosts);
			job.nested_required = Some(required);
			job.nested_streamed = job.nested_streamed.max(streamed);
		}
	}
	*remaining > 0
}
