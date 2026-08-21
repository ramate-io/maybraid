//! Per-`T` job begin with shared Presence / Desired admission.
//!
//! One host scan builds candidate lists; admission walks those lists in class order.
//! Active begin quota is rolled into Desired (begins only start desired-level jobs).
//! Each admitted job charges shared begin weight for **prefilled** primitives only
//! ([`LodChunkFulfillBudget::begin_prefill_weights_per_job`]); lazy tails drain later.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::lod_ref::{point_bounds, LodNode, LodNodeBounds, LodNodePose, LodRef};
use crate::scene::host::{
	lod_level_roots_entity, nested_host_parent_allows_refresh, parent_host_desired_or_high,
	LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::super::super::viewer::LodViewer;
use super::schedule::{admit_begin, charge_begin_weight, LevelBand};
use super::types::{
	FulfillClass, LodChunkBeginClock, LodChunkFulfillBudget, LodChunkFulfillment, LodCullInFlight,
	LodLevelRootPending, LodLevelRootStreamed,
};
use super::util::has_present_root;
use crate::scene::chunk::materialize_front;

#[derive(Clone, Copy)]
struct BeginCandidate {
	host: Entity,
	roots_entity: Entity,
	level: LodSceneLevel,
	cold: bool,
	parent_desired: LodSceneLevel,
}

fn roll_active_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock.desired_remaining.saturating_add(clock.active_remaining);
	clock.active_remaining = 0;
}

fn roll_presence_into_desired(clock: &mut LodChunkBeginClock) {
	clock.desired_remaining = clock.desired_remaining.saturating_add(clock.presence_remaining);
	clock.presence_remaining = 0;
}

fn roll_desired_into_presence(clock: &mut LodChunkBeginClock) {
	clock.presence_remaining = clock.presence_remaining.saturating_add(clock.desired_remaining);
	clock.desired_remaining = 0;
}

/// Start a pending root + queue from [`LodLevelSpawnRequest`].
pub fn begin_chunk_lod_fulfill<T: Component + LodScene>(
	mut commands: Commands,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
	mut begin_clock: ResMut<LodChunkBeginClock>,
	budget: Res<LodChunkFulfillBudget>,
	mut host_sets: ParamSet<(
		Query<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<T>)>,
		Query<&'static T, With<LodSceneHost>>,
	)>,
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

	roll_active_into_desired(&mut begin_clock);

	let mut presence_near = Vec::new();
	let mut presence_far = Vec::new();
	let mut desired_near = Vec::new();
	let mut desired_far = Vec::new();

	for (host, request) in host_sets.p0().iter() {
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
		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_bags) else {
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
			&child_of,
			&host_levels,
			&root_keys,
			&children_q,
			&level_roots_bags,
			&visibilities,
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
			cold = !has_present_root(root_children, &root_keys, &wants_cull);
		}

		let parent_desired = parent_host_desired_or_high(host, &child_of, &host_levels);
		let candidate =
			BeginCandidate { host, roots_entity, level: request.level, cold, parent_desired };
		let near = LevelBand::from_level(request.level).is_near();
		match (cold, near) {
			(true, true) => presence_near.push(candidate),
			(true, false) => presence_far.push(candidate),
			(false, true) => desired_near.push(candidate),
			(false, false) => desired_far.push(candidate),
		}
	}

	let presence_first = matches!(begin_clock.first_class, FulfillClass::Presence);
	if presence_first {
		admit_candidates(
			&presence_near,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&presence_far,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		roll_presence_into_desired(&mut begin_clock);
		admit_candidates(
			&desired_near,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&desired_far,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
	} else {
		admit_candidates(
			&desired_near,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&desired_far,
			FulfillClass::Desired,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		roll_desired_into_presence(&mut begin_clock);
		admit_candidates(
			&presence_near,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
		admit_candidates(
			&presence_far,
			FulfillClass::Presence,
			&mut commands,
			&lod_ref,
			&mut begin_clock,
			&budget,
			&mut host_sets,
		);
	}
}

fn admit_candidates<T: Component + LodScene>(
	candidates: &[BeginCandidate],
	class: FulfillClass,
	commands: &mut Commands,
	lod_ref: &LodRef,
	begin_clock: &mut LodChunkBeginClock,
	budget: &LodChunkFulfillBudget,
	host_sets: &mut ParamSet<(
		Query<(Entity, &LodLevelSpawnRequest), (With<LodSceneHost>, With<T>)>,
		Query<&'static T, With<LodSceneHost>>,
	)>,
) {
	for candidate in candidates {
		if !admit_begin(begin_clock, class) {
			break;
		}

		let chunk = {
			let scenes = host_sets.p1();
			let Ok(scene) = scenes.get(candidate.host) else {
				continue;
			};
			scene.scene_chunks_with_level(lod_ref, candidate.level)
		};
		let expected = chunk.total_primitives();
		let mut queue = chunk.into_fulfill_queue();
		let prefill = begin_clock.weight_remaining.min(budget.begin_prefill_weights_per_job);
		let begin_weight = materialize_front(&mut queue, prefill);
		charge_begin_weight(begin_clock, begin_weight.max(1));

		let level = candidate.level;
		let cold = candidate.cold;
		let initial_vis = if cold { Visibility::Inherited } else { Visibility::Hidden };
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
			host: candidate.host,
			parent_desired: candidate.parent_desired,
			nested_streamed: 0,
			nested_required: None,
		};
		if fulfillment.is_content_complete() {
			commands.entity(root_entity).insert(LodLevelRootStreamed);
		}
		commands.entity(root_entity).insert(fulfillment);
		commands.entity(candidate.roots_entity).add_child(root_entity);
		commands.entity(candidate.host).remove::<LodLevelSpawnRequest>();
	}
}
