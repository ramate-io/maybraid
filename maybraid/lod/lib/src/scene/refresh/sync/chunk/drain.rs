//! Global weighted drain with Presence / Level priority and round-robin fairness.

use std::time::Instant;

use bevy::prelude::*;
use bevy::scene::prelude::bsn;

use crate::scene::host::LodLevelRoot;
use crate::scene::level::LodSceneLevel;

use super::schedule::{for_each_rr, split_presence_level, LevelBand};
use super::types::{
	LodChunkBudgetClock, LodChunkDrainCursor, LodChunkFulfillBudget, LodChunkFulfillment,
	LodLevelRootPending, LodLevelRootStreamed, LodWantsCull,
};
use super::util::ms;

#[derive(Default)]
struct JobBuckets {
	presence: Vec<Entity>,
	high: Vec<Entity>,
	medium: Vec<Entity>,
	low: Vec<Entity>,
	ultra: Vec<Entity>,
	other: Vec<Entity>,
}

impl JobBuckets {
	fn push(&mut self, entity: Entity, cold: bool, level: LodSceneLevel) {
		if cold {
			self.presence.push(entity);
			return;
		}
		match LevelBand::from_level(level) {
			LevelBand::High => self.high.push(entity),
			LevelBand::Medium => self.medium.push(entity),
			LevelBand::Low => self.low.push(entity),
			LevelBand::UltraLow => self.ultra.push(entity),
			LevelBand::Other => self.other.push(entity),
		}
	}
}

struct DrainStats {
	spawned: u32,
	weight_spent: u32,
	newly_streamed: u32,
	jobs_touched: u32,
}

/// Drain weighted primitives under [`LodChunkBudgetClock`].
///
/// Budget is split ~⅓ Presence (cold jobs) / ~⅔ Level (warm, High→far). Frame
/// parity chooses which class runs first; leftovers roll into the second class.
/// Within each list, a round-robin cursor avoids ECS-order pinning.
pub fn drain_chunk_lod_fulfill(
	mut commands: Commands,
	mut clock: ResMut<LodChunkBudgetClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut cursor: ResMut<LodChunkDrainCursor>,
	mut jobs: Query<
		(Entity, &LodLevelRoot, &mut LodChunkFulfillment),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
	>,
) {
	let t0 = Instant::now();
	let total = clock.spawn_remaining;
	if total == 0 {
		return;
	}

	let mut buckets = JobBuckets::default();
	for (entity, root, job) in &jobs {
		if job.queue.is_empty() {
			continue;
		}
		buckets.push(entity, job.cold, root.0);
	}

	let (presence_share, level_share) = split_presence_level(total);
	let presence_first = cursor.frame % 2 == 0;
	let mut stats = DrainStats {
		spawned: 0,
		weight_spent: 0,
		newly_streamed: 0,
		jobs_touched: 0,
	};

	let mut remaining = if presence_first {
		presence_share
	} else {
		level_share
	};

	if presence_first {
		drain_presence(
			&mut commands,
			&mut jobs,
			&buckets.presence,
			&mut cursor.presence,
			&mut remaining,
			&mut stats,
		);
		remaining = remaining.saturating_add(level_share);
		drain_level_bands(
			&mut commands,
			&mut jobs,
			&buckets,
			&mut cursor,
			&mut remaining,
			&mut stats,
		);
	} else {
		drain_level_bands(
			&mut commands,
			&mut jobs,
			&buckets,
			&mut cursor,
			&mut remaining,
			&mut stats,
		);
		remaining = remaining.saturating_add(presence_share);
		drain_presence(
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
			 jobs_touched={} newly_streamed={} presence_first={presence_first} \
			 queue_cmds={:.2}ms (rest={remaining})",
			stats.spawned,
			stats.weight_spent,
			budget.spawn_weights_per_frame,
			stats.jobs_touched,
			stats.newly_streamed,
			ms(t0),
		);
	}
}

fn drain_presence(
	commands: &mut Commands,
	jobs: &mut Query<
		(Entity, &LodLevelRoot, &mut LodChunkFulfillment),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
	>,
	presence: &[Entity],
	cursor: &mut u32,
	remaining: &mut u32,
	stats: &mut DrainStats,
) {
	for_each_rr(presence, cursor, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
}

fn drain_level_bands(
	commands: &mut Commands,
	jobs: &mut Query<
		(Entity, &LodLevelRoot, &mut LodChunkFulfillment),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
	>,
	buckets: &JobBuckets,
	cursor: &mut LodChunkDrainCursor,
	remaining: &mut u32,
	stats: &mut DrainStats,
) {
	if *remaining == 0 {
		return;
	}
	for_each_rr(&buckets.high, &mut cursor.high, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
	if *remaining == 0 {
		return;
	}
	for_each_rr(&buckets.medium, &mut cursor.medium, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
	if *remaining == 0 {
		return;
	}
	for_each_rr(&buckets.low, &mut cursor.low, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
	if *remaining == 0 {
		return;
	}
	for_each_rr(&buckets.ultra, &mut cursor.ultra, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
	if *remaining == 0 {
		return;
	}
	for_each_rr(&buckets.other, &mut cursor.other, |&entity| {
		drain_one(commands, jobs, entity, remaining, stats)
	});
}

/// Drain one job while budget remains. Returns `false` when the frame budget is empty.
fn drain_one(
	commands: &mut Commands,
	jobs: &mut Query<
		(Entity, &LodLevelRoot, &mut LodChunkFulfillment),
		(With<LodLevelRootPending>, Without<LodWantsCull>),
	>,
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
