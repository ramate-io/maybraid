use std::collections::{BTreeMap, HashSet, VecDeque};

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use intelligence_lod::{
	IntelligenceBand, IntelligenceDueEntities, IntelligenceLod, IntelligencePriority,
};
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{SpotCandidate, SpotSubject, SpottingObserveLimits, SpottingUser};

use crate::candidate::ProbeCandidate;
use crate::discover::{discover_subjects, merge_due_contacts, next_discovery_interval};
use crate::probe::probe_candidate_visibility;

fn forget_invalid_contacts(
	now: f32,
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: &Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
) {
	for (_, mut user, _) in spotters.iter_mut() {
		user.forget_stale(now);
		user.contacts.retain(|entity, _| subjects.get(*entity).is_ok());
	}
}

fn collect_due_discoverers(
	now: f32,
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	due_entities: &mut IntelligenceDueEntities,
) {
	due_entities.clear();
	for (entity, user, _) in spotters.iter_mut() {
		if now >= user.next_discovery_at {
			due_entities.push(entity);
		}
	}
}

fn fair_far_spotter(
	due: &[Entity],
	spotters: &mut Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
) -> Option<Entity> {
	due.iter().copied().find(|entity| {
		spotters.get_mut(*entity).is_ok_and(|(_, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	})
}

fn schedule_discovery(
	now: f32,
	user: &mut SpottingUser,
	band: IntelligenceBand,
	spotter_entity: Entity,
	fair: Option<Entity>,
) -> (bool, f32) {
	let discovery_due = now >= user.next_discovery_at;
	let next_interval = next_discovery_interval(user);
	let run_discovery =
		discovery_due && (band != IntelligenceBand::Far || Some(spotter_entity) == fair);
	(run_discovery, next_interval)
}

fn advance_discovery_clock(
	now: f32,
	user: &mut SpottingUser,
	band: IntelligenceBand,
	run_discovery: bool,
	discovery_due: bool,
	next_interval: f32,
) {
	if run_discovery {
		// Fair Far uses the raw interval so the reserve is not immediately
		// stretched away; Mid / Near still use `interval_scale`.
		let scale = if band == IntelligenceBand::Far { 1.0 } else { band.interval_scale() };
		user.next_discovery_at = now + next_interval * scale;
	} else if discovery_due {
		user.next_discovery_at = now + next_interval * IntelligenceBand::Far.interval_scale();
	}
}

/// Scratch for the shared discovery and retained-contact visibility executor.
#[derive(Default)]
#[doc(hidden)]
pub struct ProbeBatch {
	candidates: BTreeMap<Entity, ProbeCandidate>,
	ranked: Vec<SpotCandidate>,
	grants: Vec<usize>,
}

impl ProbeBatch {
	#[allow(clippy::too_many_arguments)]
	fn execute(
		&mut self,
		user: &mut SpottingUser,
		now: f32,
		observer: Vec3,
		discovery: bool,
		band: IntelligenceBand,
		sample_cap: usize,
		spatial: &SpatialQuery,
		fixed_filter: &SpatialQueryFilter,
		subjects: &Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
		transforms: &TransformHelper,
	) -> usize {
		merge_due_contacts(user, now, observer, subjects, transforms, &mut self.candidates);
		self.ranked.clear();
		self.ranked.extend(self.candidates.values().map(|candidate| candidate.rank));
		SpotCandidate::rank(&mut self.ranked);
		let candidate_budget = band.scale_count(user.settings.candidate_budget);
		let vision_samples = band.scale_count(user.settings.vision_samples).min(sample_cap);
		SpotCandidate::apply_budget(&mut self.ranked, candidate_budget);
		SpotCandidate::allocate_sample_budget_into(
			&self.ranked,
			candidate_budget,
			vision_samples,
			&mut self.grants,
		);
		let allocated = self.grants.iter().sum();
		let discovery_sample_cursor =
			if discovery && allocated > 0 { user.advance_sample_cursor() } else { 0 };
		probe_candidate_visibility(
			user,
			now,
			observer,
			discovery_sample_cursor,
			&self.candidates,
			&self.ranked,
			&self.grants,
			spatial,
			fixed_filter,
			subjects,
			transforms,
		);
		allocated
	}
}

/// Shared service ledger for one Update, reset by [`prepare_spotting`].
/// Plugin registration initializes it; manual registration must do the same.
#[derive(Resource, Default)]
#[doc(hidden)]
pub struct SpottingFrameState {
	observed: HashSet<Entity>,
	discovery_observers: usize,
	discovery_samples: usize,
	retained_observers: usize,
	retained_samples: usize,
}

/// Expire contacts and start a fresh bounded service ledger, independently of
/// whether discovery or retained budgets are zero in this Update.
pub fn prepare_spotting(
	time: Res<Time>,
	mut frame: ResMut<SpottingFrameState>,
	mut spotters: Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
) {
	let _span = trace_span!(target: "maybraid.spotting.profile", "spotting_prepare").entered();
	frame.observed.clear();
	frame.discovery_observers = 0;
	frame.discovery_samples = 0;
	frame.retained_observers = 0;
	frame.retained_samples = 0;
	forget_invalid_contacts(time.elapsed_secs(), &mut spotters, &subjects);
}

/// Bounded broadphase/hint discovery, including the observer's due contacts.
/// Must run after Prepare and before Refresh within the parent Observe set.
pub fn discover_spotting(
	spatial: SpatialQuery,
	time: Res<Time>,
	priority: Res<IntelligencePriority>,
	limits: Res<SpottingObserveLimits>,
	mut frame: ResMut<SpottingFrameState>,
	mut spotters: Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
	parents: Query<&ChildOf>,
	transforms: TransformHelper,
	mut due_entities: Local<IntelligenceDueEntities>,
	mut batch: Local<ProbeBatch>,
) {
	let now = time.elapsed_secs();
	let animated_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Animated);
	let fixed_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	collect_due_discoverers(now, &mut spotters, &mut due_entities);
	let due = due_entities.finish(&priority);
	let fair = fair_far_spotter(due, &mut spotters);
	let span = trace_span!(target: "maybraid.spotting.profile", "spotting_discovery",
		due_observers = due.len(), observer_cap = limits.max_observers_per_tick,
		sample_cap = limits.max_discovery_samples_per_tick);
	let _guard = span.enter();
	for &entity in due {
		let remaining =
			limits.max_discovery_samples_per_tick.saturating_sub(frame.discovery_samples);
		if frame.discovery_observers >= limits.max_observers_per_tick || remaining == 0 {
			break;
		}
		if frame.observed.contains(&entity) {
			continue;
		}
		let Ok((_, mut user, mut lod)) = spotters.get_mut(entity) else { continue };
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		let Ok(transform) = transforms.compute_global_transform(entity) else { continue };
		let observer = transform.translation() + transform.rotation() * user.eye_offset;
		let discovery_due = now >= user.next_discovery_at;
		let (run_discovery, interval) = schedule_discovery(now, &mut user, band, entity, fair);
		advance_discovery_clock(now, &mut user, band, run_discovery, discovery_due, interval);
		if !run_discovery {
			continue;
		}
		frame.discovery_observers += 1;
		frame.observed.insert(entity);
		batch.candidates.clear();
		discover_subjects(
			entity,
			&user,
			now,
			observer,
			&spatial,
			&animated_filter,
			&subjects,
			&parents,
			&transforms,
			&mut batch.candidates,
		);
		frame.discovery_samples += batch.execute(
			&mut user,
			now,
			observer,
			true,
			band,
			remaining,
			&spatial,
			&fixed_filter,
			&subjects,
			&transforms,
		);
		if let Some(lod) = lod.as_deref_mut() {
			if lod.skips != 0 {
				lod.skips = 0;
			}
		}
	}
	let _counts = trace_span!(target: "maybraid.spotting.profile", "spotting_discovery_grants",
        observers = frame.discovery_observers, allocated_samples = frame.discovery_samples)
	.entered();
}

/// Refresh only retained contacts in a persistent fair round. The independent
/// observer/sample caps reset each Update; elapsed time never earns extra credit.
pub fn refresh_spotting(
	spatial: SpatialQuery,
	time: Res<Time>,
	priority: Res<IntelligencePriority>,
	limits: Res<SpottingObserveLimits>,
	mut frame: ResMut<SpottingFrameState>,
	mut spotters: Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: Query<(Entity, &SpotSubject, Option<&avian3d::prelude::LinearVelocity>)>,
	transforms: TransformHelper,
	mut due_entities: Local<IntelligenceDueEntities>,
	mut batch: Local<ProbeBatch>,
	mut retained_round: Local<VecDeque<Entity>>,
) {
	let now = time.elapsed_secs();
	if limits.max_retained_observers_per_tick == 0 || limits.max_retained_samples_per_tick == 0 {
		return;
	}
	if retained_round.is_empty() {
		due_entities.clear();
		for (entity, user, _) in &mut spotters {
			if !frame.observed.contains(&entity)
				&& user.contacts.values().any(|contact| contact.is_due(now))
			{
				due_entities.push(entity);
			}
		}
		retained_round.extend(due_entities.finish(&priority).iter().copied());
	}
	let span = trace_span!(target: "maybraid.spotting.profile", "spotting_retained_refresh",
		queued_observers = retained_round.len(), observer_cap = limits.max_retained_observers_per_tick,
		sample_cap = limits.max_retained_samples_per_tick);
	let _guard = span.enter();
	let fixed_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	while frame.retained_observers < limits.max_retained_observers_per_tick {
		let remaining = limits.max_retained_samples_per_tick.saturating_sub(frame.retained_samples);
		// Preserve the unserved head when this Update's sample credit is exhausted.
		if remaining == 0 {
			break;
		}
		let Some(entity) = retained_round.pop_front() else { break };
		if frame.observed.contains(&entity) {
			continue;
		}
		let Ok((_, mut user, lod)) = spotters.get_mut(entity) else { continue };
		if !user.contacts.values().any(|contact| contact.is_due(now)) {
			continue;
		}
		let Ok(transform) = transforms.compute_global_transform(entity) else { continue };
		let observer = transform.translation() + transform.rotation() * user.eye_offset;
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		batch.candidates.clear();
		frame.retained_samples += batch.execute(
			&mut user,
			now,
			observer,
			false,
			band,
			remaining,
			&spatial,
			&fixed_filter,
			&subjects,
			&transforms,
		);
		frame.retained_observers += 1;
		frame.observed.insert(entity);
	}
	let _counts = trace_span!(target: "maybraid.spotting.profile", "spotting_retained_grants",
        observers = frame.retained_observers, allocated_samples = frame.retained_samples)
	.entered();
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::candidate::ProbeCandidate;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
	use spotting_intelligence::{
		InterestLayers, SpotBounds, SpotDirective, SpottedContact, SpottingHint,
		SpottingObserveLimits, SpottingSettings,
	};

	fn observe_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.add_plugins(crate::SpottingAvianPlugin);
		app.finish();
		app
	}

	fn character_user() -> SpottingUser {
		SpottingUser::new(Vec3::Y * 1.6, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)])
			.with_settings(SpottingSettings::new(4, 4, 2.0))
	}

	fn spawn_subject(app: &mut App, at: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Transform::from_translation(at),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id()
	}

	fn spawn_animated_subject(app: &mut App, at: Vec3) -> Entity {
		let entity = app
			.world_mut()
			.spawn((
				Transform::from_translation(at),
				GlobalTransform::from_translation(at),
				RigidBody::Static,
				Collider::sphere(0.5),
				PhysicsInteractionLayer::animated_layers(),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id();
		app.update();
		entity
	}

	#[test]
	fn merge_uses_highest_priority_policy() -> anyhow::Result<()> {
		let subject = Entity::from_bits(1);
		let low = SpotDirective {
			priority: 1,
			respot_interval_secs: 0.5,
			max_samples_per_subject: 9,
			..SpotDirective::default()
		};
		let high = SpotDirective {
			priority: 3,
			respot_interval_secs: 0.1,
			max_samples_per_subject: 2,
			..SpotDirective::default()
		};
		let mut candidate = ProbeCandidate::new(subject, low, 1.0, 2.0, false, 9);
		candidate.merge(ProbeCandidate::new(subject, high, 1.0, 2.0, true, 9));
		assert_eq!(candidate.rank.directive_priority, 3);
		assert_eq!(candidate.rank.max_samples, 2);
		assert!(candidate.rank.known);
		assert_eq!(candidate.respot_interval_secs, 0.1);
		Ok(())
	}

	#[test]
	fn explicit_hint_reaches_visibility_without_a_broadphase_collider() -> anyhow::Result<()> {
		let mut app = observe_app();
		let subject = app
			.world_mut()
			.spawn((
				Transform::from_xyz(4.0, 0.9, 0.0),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			))
			.id();
		let mut user = character_user();
		user.hint(subject, SpottingHint::new(2));
		let observer = app.world_mut().spawn((Transform::default(), user)).id();

		app.update();

		let user = app
			.world()
			.get::<SpottingUser>(observer)
			.ok_or_else(|| anyhow::anyhow!("observer lost SpottingUser"))?;
		assert!(user.contacts.contains_key(&subject));
		Ok(())
	}

	#[test]
	fn far_skips_animated_broadphase_unless_fair() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits {
			max_observers_per_tick: 1,
			max_retained_observers_per_tick: 0,
			..default()
		});
		let subject = spawn_animated_subject(&mut app, Vec3::X * 4.0);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();
		let near = app
			.world_mut()
			.spawn((
				Transform::from_xyz(0.0, 0.0, 1.0),
				character_user(),
				IntelligenceLod::missing(),
			))
			.id();
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

		app.update();

		anyhow::ensure!(
			app.world()
				.get::<SpottingUser>(near)
				.is_some_and(|user| user.contacts.contains_key(&subject))
		);
		anyhow::ensure!(
			app.world()
				.get::<SpottingUser>(far)
				.is_some_and(|user| !user.contacts.contains_key(&subject))
		);
		Ok(())
	}

	#[test]
	fn far_broadphase_still_runs_at_fairness_cap() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits {
			max_observers_per_tick: 1,
			max_retained_observers_per_tick: 0,
			..default()
		});
		let subject = spawn_animated_subject(&mut app, Vec3::X * 4.0);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod {
					band: IntelligenceBand::Far,
					skips: IntelligenceLod::FAIRNESS_CAP,
				},
			))
			.id();

		app.update();

		anyhow::ensure!(
			app.world()
				.get::<SpottingUser>(far)
				.is_some_and(|user| user.contacts.contains_key(&subject))
		);
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
		Ok(())
	}

	#[test]
	fn forget_stale_still_runs_on_far_we_do_not_discover() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits {
			max_observers_per_tick: 1,
			max_retained_observers_per_tick: 0,
			..default()
		});
		let stale = spawn_subject(&mut app, Vec3::X * 3.0);
		let keep = spawn_subject(&mut app, Vec3::X * 5.0);
		let mut far_user = character_user();
		far_user.contacts.insert(
			stale,
			SpottedContact::new(stale, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, -4.0, 0.1),
		);
		let mut near_user = character_user();
		near_user.contacts.insert(
			keep,
			SpottedContact::new(keep, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 0.0, 0.1),
		);
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				far_user,
				IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
			))
			.id();
		let near = app
			.world_mut()
			.spawn((Transform::from_xyz(0.0, 0.0, 1.0), near_user, IntelligenceLod::missing()))
			.id();
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
		app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

		app.update();

		anyhow::ensure!(
			app.world()
				.get::<SpottingUser>(far)
				.is_some_and(|user| !user.contacts.contains_key(&stale))
		);
		anyhow::ensure!(
			app.world()
				.get::<SpottingUser>(near)
				.is_some_and(|user| user.contacts.contains_key(&keep))
		);
		Ok(())
	}

	#[test]
	fn far_discovery_skip_stretches_the_interval() -> anyhow::Result<()> {
		let mut app = observe_app();
		let far = app
			.world_mut()
			.spawn((
				Transform::default(),
				character_user(),
				IntelligenceLod { band: IntelligenceBand::Far, skips: 3 },
			))
			.id();

		app.update();

		let interval = SpotDirective::new(InterestLayers::CHARACTER, 20.0).discovery_interval_secs;
		let next = app
			.world()
			.get::<SpottingUser>(far)
			.ok_or_else(|| anyhow::anyhow!("far lost SpottingUser"))?
			.next_discovery_at;
		anyhow::ensure!(next >= interval * IntelligenceBand::Far.interval_scale() - 1e-3);
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 3));
		Ok(())
	}
}

#[cfg(test)]
#[path = "observe/retained_tests.rs"]
mod retained_tests;

#[cfg(test)]
#[path = "observe/cadence_tests.rs"]
mod cadence_tests;
