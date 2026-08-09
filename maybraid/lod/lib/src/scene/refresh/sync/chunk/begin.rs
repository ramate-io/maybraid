//! Per-`T` job begin with shared Presence / Desired admission.
//!
//! Active begin quota is rolled into Desired (begins only start desired-level jobs;
//! Active spend is a drain concern for warm-hold queues).

use std::time::Instant;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::lod_ref::{point_bounds, LodNode, LodNodeBounds, LodNodePose, LodRef};
use crate::scene::host::{
	nested_host_parent_allows_refresh, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest,
	LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::super::super::viewer::LodViewer;
use super::schedule::{admit_begin, LevelBand};
use super::types::{
	FulfillClass, LodChunkBeginClock, LodChunkFulfillDiag, LodChunkFulfillment, LodCullInFlight,
	LodLevelRootPending, LodLevelRootStreamed,
};
use super::util::{has_present_root, ms, roots_bag_entity};

/// Which hosts a begin pass may admit.
#[derive(Clone, Copy)]
enum BeginPass {
	PresenceNear,
	PresenceFar,
	DesiredNear,
	DesiredFar,
}

impl BeginPass {
	fn allows(self, cold: bool, level: LodSceneLevel) -> bool {
		let band = LevelBand::from_level(level);
		match self {
			Self::PresenceNear => cold && band.is_near(),
			Self::PresenceFar => cold && !band.is_near(),
			Self::DesiredNear => !cold && band.is_near(),
			Self::DesiredFar => !cold && !band.is_near(),
		}
	}

	fn class(self) -> FulfillClass {
		match self {
			Self::PresenceNear | Self::PresenceFar => FulfillClass::Presence,
			Self::DesiredNear | Self::DesiredFar => FulfillClass::Desired,
		}
	}
}

fn roll_active_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock
		.desired_remaining
		.saturating_add(clock.active_remaining);
	clock.active_remaining = 0;
}

fn roll_presence_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock
		.desired_remaining
		.saturating_add(clock.presence_remaining);
	clock.presence_remaining = 0;
}

fn roll_desired_into_presence(clock: &mut LodChunkBeginClock) {
	clock.presence_remaining = clock
		.presence_remaining
		.saturating_add(clock.desired_remaining);
	clock.desired_remaining = 0;
}

struct BeginStats {
	jobs_started: u32,
	chunks_ms_total: f64,
	spawn_ms_total: f64,
}

/// Start a pending root + queue from [`LodLevelSpawnRequest`].
pub fn begin_chunk_lod_fulfill<T: Component + LodScene>(
	mut commands: Commands,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
	mut diag: ResMut<LodChunkFulfillDiag>,
	mut begin_clock: ResMut<LodChunkBeginClock>,
	hosts: Query<(Entity, &T, &LodLevelSpawnRequest), With<LodSceneHost>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
	wants_cull: Query<(), With<LodCullInFlight>>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	children_q: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	visibilities: Query<&Visibility>,
) {
	let Ok((viewer_entity, pose, viewer_bounds)) = viewer.single() else {
		return;
	};
	let driver_bounds = viewer_bounds
		.map(|b| b.0)
		.unwrap_or_else(|| point_bounds(pose.current.translation));
	let lod_ref = pose.as_lod_ref(viewer_entity, &driver_bounds);

	let t_sys = Instant::now();
	let mut stats = BeginStats {
		jobs_started: 0,
		chunks_ms_total: 0.0,
		spawn_ms_total: 0.0,
	};

	roll_active_into_desired(&mut begin_clock);
	let presence_first = matches!(begin_clock.first_class, FulfillClass::Presence);

	if presence_first {
		for pass in [BeginPass::PresenceNear, BeginPass::PresenceFar] {
			run_begin_pass(
				pass,
				&mut commands,
				&lod_ref,
				&mut diag,
				&mut begin_clock,
				&hosts,
				&root_keys,
				&pending,
				&wants_cull,
				&child_of,
				&host_levels,
				&children_q,
				&level_roots_bags,
				&visibilities,
				&mut stats,
			);
		}
		roll_presence_into_desired(&mut begin_clock);
		for pass in [BeginPass::DesiredNear, BeginPass::DesiredFar] {
			run_begin_pass(
				pass,
				&mut commands,
				&lod_ref,
				&mut diag,
				&mut begin_clock,
				&hosts,
				&root_keys,
				&pending,
				&wants_cull,
				&child_of,
				&host_levels,
				&children_q,
				&level_roots_bags,
				&visibilities,
				&mut stats,
			);
		}
	} else {
		for pass in [BeginPass::DesiredNear, BeginPass::DesiredFar] {
			run_begin_pass(
				pass,
				&mut commands,
				&lod_ref,
				&mut diag,
				&mut begin_clock,
				&hosts,
				&root_keys,
				&pending,
				&wants_cull,
				&child_of,
				&host_levels,
				&children_q,
				&level_roots_bags,
				&visibilities,
				&mut stats,
			);
		}
		roll_desired_into_presence(&mut begin_clock);
		for pass in [BeginPass::PresenceNear, BeginPass::PresenceFar] {
			run_begin_pass(
				pass,
				&mut commands,
				&lod_ref,
				&mut diag,
				&mut begin_clock,
				&hosts,
				&root_keys,
				&pending,
				&wants_cull,
				&child_of,
				&host_levels,
				&children_q,
				&level_roots_bags,
				&visibilities,
				&mut stats,
			);
		}
	}

	if stats.jobs_started > 0 {
		info!(
			"[lod.chunk] begin_chunk_lod_fulfill: {} jobs, \
			 scene_chunks_with_level={:.2}ms queue_root_cmds={:.2}ms \
			 total={:.2}ms",
			stats.jobs_started,
			stats.chunks_ms_total,
			stats.spawn_ms_total,
			ms(t_sys)
		);
	}
}

fn run_begin_pass<T: Component + LodScene>(
	pass: BeginPass,
	commands: &mut Commands,
	lod_ref: &LodRef,
	diag: &mut LodChunkFulfillDiag,
	begin_clock: &mut LodChunkBeginClock,
	hosts: &Query<(Entity, &T, &LodLevelSpawnRequest), With<LodSceneHost>>,
	root_keys: &Query<&LodLevelRoot>,
	pending: &Query<(), With<LodLevelRootPending>>,
	wants_cull: &Query<(), With<LodCullInFlight>>,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	visibilities: &Query<&Visibility>,
	stats: &mut BeginStats,
) {
	if begin_clock.presence_remaining == 0 && begin_clock.desired_remaining == 0 {
		return;
	}

	for (host, scene, request) in hosts.iter() {
		let Ok(desired) = host_levels.get(host) else {
			continue;
		};
		if request.level != *desired {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		}

		let Ok(host_children) = children_q.get(host) else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};
		let Some(roots_entity) = roots_bag_entity(host_children, level_roots_bags) else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};

		let root_children = children_q.get(roots_entity).ok();
		let has_any_level_root = root_children.is_some_and(|children| {
			children
				.iter()
				.any(|child| root_keys.contains(child) && !wants_cull.contains(child))
		});

		if !nested_host_parent_allows_refresh(
			host,
			child_of,
			host_levels,
			root_keys,
			children_q,
			level_roots_bags,
			visibilities,
		) && has_any_level_root
		{
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		}

		let mut cold = true;
		if let Some(root_children) = root_children {
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
			cold = !has_present_root(root_children, root_keys, wants_cull);
		}

		if !pass.allows(cold, request.level) {
			continue;
		}
		if !admit_begin(begin_clock, pass.class()) {
			continue;
		}

		let t_chunks = Instant::now();
		let chunk = scene.scene_chunks_with_level(lod_ref, request.level);
		let queue = chunk.into_primitives();
		let chunks_ms = ms(t_chunks);
		stats.chunks_ms_total += chunks_ms;
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
			cold,
		};
		if fulfillment.is_content_complete() {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		commands.entity(root_entity).insert(fulfillment);
		commands.entity(roots_entity).add_child(root_entity);
		commands.entity(host).remove::<LodLevelSpawnRequest>();
		stats.spawn_ms_total += ms(t_spawn);
		stats.jobs_started += 1;

		info!(
			"[lod.chunk] begin job host={host:?} level={level:?} cold={cold}: \
			 scene_chunks_with_level={chunks_ms:.2}ms expected={expected} \
			 queue_weight={queue_weight} queue_root_cmds={:.2}ms",
			ms(t_spawn)
		);
	}
}
