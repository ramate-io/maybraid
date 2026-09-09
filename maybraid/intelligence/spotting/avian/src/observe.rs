use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use avian3d::prelude::{Collider, LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use intelligence_lod::{due_by_rank, IntelligenceBand, IntelligenceLod, IntelligencePriority};
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{
	allocate_sample_budget, apply_candidate_budget, rank_candidates, SpotCandidate,
	SpotContactView, SpotDirective, SpotSubject, SpottedContact, SpottingObserveLimits,
	SpottingUser,
};

use crate::clear_segment;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProbeCandidate {
	rank: SpotCandidate,
	respot_interval_secs: f32,
}

impl ProbeCandidate {
	fn new(
		subject: Entity,
		directive: SpotDirective,
		salience: f32,
		distance: f32,
		known: bool,
		available_samples: usize,
	) -> Self {
		Self {
			rank: SpotCandidate {
				subject,
				directive_priority: directive.priority,
				salience: if salience.is_finite() { salience } else { 0.0 },
				distance,
				known,
				max_samples: directive.max_samples_per_subject.min(available_samples),
			},
			respot_interval_secs: directive.respot_interval_secs.max(0.0),
		}
	}

	fn merge(&mut self, other: Self) {
		if other.rank.directive_priority > self.rank.directive_priority {
			*self = other;
			return;
		}
		if other.rank.directive_priority == self.rank.directive_priority {
			self.rank.max_samples = self.rank.max_samples.max(other.rank.max_samples);
			self.respot_interval_secs = self.respot_interval_secs.min(other.respot_interval_secs);
		}
		self.rank.known |= other.rank.known;
	}
}

fn merge_candidate(candidates: &mut BTreeMap<Entity, ProbeCandidate>, candidate: ProbeCandidate) {
	match candidates.entry(candidate.rank.subject) {
		Entry::Vacant(entry) => {
			entry.insert(candidate);
		}
		Entry::Occupied(mut entry) => entry.get_mut().merge(candidate),
	}
}

fn directive_satisfied(
	directive: SpotDirective,
	now: f32,
	observer: Vec3,
	user: &SpottingUser,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	transforms: &TransformHelper,
) -> bool {
	directive.is_satisfied(
		now,
		user.contacts.values().filter_map(|contact| {
			let Ok((_, subject, _)) = subjects.get(contact.subject) else {
				return None;
			};
			let Ok(transform) = transforms.compute_global_transform(contact.subject) else {
				return None;
			};
			Some(SpotContactView {
				contact,
				layers: subject.layers,
				distance: transform.translation().distance(observer),
			})
		}),
	)
}

fn next_discovery_interval(user: &SpottingUser) -> f32 {
	user.directives
		.iter()
		.map(|directive| directive.discovery_interval_secs.max(0.0))
		.reduce(f32::min)
		.unwrap_or(0.25)
}

fn discover_subjects(
	spotter_entity: Entity,
	user: &SpottingUser,
	now: f32,
	observer: Vec3,
	spatial: &SpatialQuery,
	animated_filter: &SpatialQueryFilter,
	subjects: &Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: &Query<&ChildOf>,
	transforms: &TransformHelper,
	candidates: &mut BTreeMap<Entity, ProbeCandidate>,
) {
	for &directive in &user.directives {
		if directive.desired_count == 0
			|| directive_satisfied(directive, now, observer, user, subjects, transforms)
		{
			continue;
		}
		let range = directive.range.max(0.0);
		if range == 0.0 || !range.is_finite() {
			continue;
		}
		let sphere = Collider::sphere(range);
		for entity in
			spatial.shape_intersections(&sphere, observer, Quat::IDENTITY, animated_filter)
		{
			if entity == spotter_entity {
				continue;
			}
			let mut subject_entity = entity;
			let Some((subject_entity, subject, _)) = (loop {
				if let Ok(subject) = subjects.get(subject_entity) {
					break Some(subject);
				}
				let Ok(parent) = parents.get(subject_entity) else {
					break None;
				};
				subject_entity = parent.parent();
			}) else {
				continue;
			};
			if subject_entity == spotter_entity {
				continue;
			}
			let Ok(transform) = transforms.compute_global_transform(subject_entity) else {
				continue;
			};
			let distance = transform.translation().distance(observer);
			if !directive.matches(subject.layers, distance) {
				continue;
			}
			let known = user.contacts.get(&subject_entity);
			if known.is_some_and(|contact| {
				contact.is_fresh(now, directive.freshness_secs) && !contact.is_due(now)
			}) {
				continue;
			}
			merge_candidate(
				candidates,
				ProbeCandidate::new(
					subject_entity,
					directive,
					subject.salience,
					distance,
					known.is_some(),
					subject.bounds.sample_count(),
				),
			);
		}
	}

	for (&subject_entity, hint) in &user.hints {
		if subject_entity == spotter_entity {
			continue;
		}
		let Ok((entity, subject, _)) = subjects.get(subject_entity) else {
			continue;
		};
		let Ok(transform) = transforms.compute_global_transform(entity) else {
			continue;
		};
		let distance = transform.translation().distance(observer);
		let known = user.contacts.get(&subject_entity);
		for &directive in &user.directives {
			if directive.desired_count == 0 || !directive.matches(subject.layers, distance) {
				continue;
			}
			if known.is_some_and(|contact| {
				contact.is_fresh(now, directive.freshness_secs) && !contact.is_due(now)
			}) {
				continue;
			}
			let mut candidate = ProbeCandidate::new(
				subject_entity,
				directive,
				subject.salience,
				distance,
				known.is_some(),
				subject.bounds.sample_count(),
			);
			candidate.rank.directive_priority =
				candidate.rank.directive_priority.saturating_add(hint.priority());
			merge_candidate(candidates, candidate);
		}
	}
}

/// Discover, rank, and visibility-probe subjects. Far skips Animated broadphase
/// unless a fairness reserve is due. `forget_stale` still runs on everyone.
/// `skips` reset only when discovery ran, matching `replan_movement`.
pub fn observe_spotting(
	spatial: SpatialQuery,
	time: Res<Time>,
	priority: Res<IntelligencePriority>,
	limits: Res<SpottingObserveLimits>,
	mut spotters: Query<(Entity, &mut SpottingUser, Option<&mut IntelligenceLod>)>,
	subjects: Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: Query<&ChildOf>,
	transforms: TransformHelper,
) {
	let now = time.elapsed_secs();
	let animated_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Animated);
	let fixed_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);

	for (_, mut user, _) in &mut spotters {
		user.forget_stale(now);
		user.contacts.retain(|entity, _| subjects.get(*entity).is_ok());
	}

	let mut due: Vec<Entity> = spotters
		.iter_mut()
		.filter_map(|(entity, user, _)| {
			let due = now >= user.next_discovery_at
				|| user.contacts.values().any(|contact| contact.is_due(now));
			due.then_some(entity)
		})
		.collect();
	due_by_rank(&mut due, &priority);
	let fair = due.iter().copied().find(|entity| {
		spotters.get_mut(*entity).is_ok_and(|(_, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	});

	let mut remaining = limits.max_observers_per_tick;
	for spotter_entity in due {
		if remaining == 0 {
			break;
		}
		let Ok((_, mut user, mut lod)) = spotters.get_mut(spotter_entity) else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		let Ok(spotter_transform) = transforms.compute_global_transform(spotter_entity) else {
			continue;
		};
		let observer =
			spotter_transform.translation() + spotter_transform.rotation() * user.eye_offset;
		let discovery_due = now >= user.next_discovery_at;
		let mut candidates = BTreeMap::new();
		let next_interval = next_discovery_interval(&user);
		let run_discovery =
			discovery_due && (band != IntelligenceBand::Far || Some(spotter_entity) == fair);
		let discovery_sample_cursor = if run_discovery { user.advance_sample_cursor() } else { 0 };

		if run_discovery {
			remaining -= 1;
			discover_subjects(
				spotter_entity,
				&user,
				now,
				observer,
				&spatial,
				&animated_filter,
				&subjects,
				&parents,
				&transforms,
				&mut candidates,
			);
			// Fair Far uses the raw interval so the reserve is not immediately
			// stretched away; Mid / Near still use `interval_scale`.
			let scale = if band == IntelligenceBand::Far { 1.0 } else { band.interval_scale() };
			user.next_discovery_at = now + next_interval * scale;
		} else if discovery_due {
			user.next_discovery_at = now + next_interval * IntelligenceBand::Far.interval_scale();
		}

		for contact in user.contacts.values().filter(|contact| contact.is_due(now)) {
			let Ok((entity, subject, _)) = subjects.get(contact.subject) else {
				continue;
			};
			let Ok(transform) = transforms.compute_global_transform(entity) else {
				continue;
			};
			let distance = transform.translation().distance(observer);
			for &directive in &user.directives {
				if !directive.matches(subject.layers, distance) {
					continue;
				}
				merge_candidate(
					&mut candidates,
					ProbeCandidate::new(
						entity,
						directive,
						subject.salience,
						distance,
						true,
						subject.bounds.sample_count(),
					),
				);
			}
		}

		let mut ranked: Vec<SpotCandidate> =
			candidates.values().map(|candidate| candidate.rank).collect();
		rank_candidates(&mut ranked);
		let candidate_budget = band.scale_count(user.settings.candidate_budget);
		let vision_samples = band.scale_count(user.settings.vision_samples);
		apply_candidate_budget(&mut ranked, candidate_budget);
		let grants = allocate_sample_budget(&ranked, candidate_budget, vision_samples);

		for (candidate, sample_budget) in ranked.into_iter().zip(grants) {
			if sample_budget == 0 {
				continue;
			}
			let Some(policy) = candidates.get(&candidate.subject) else {
				continue;
			};
			let Ok((entity, subject, velocity)) = subjects.get(candidate.subject) else {
				continue;
			};
			let Ok(transform) = transforms.compute_global_transform(entity) else {
				continue;
			};
			let samples = subject.bounds.samples(observer, transform.translation());
			if samples.is_empty() {
				continue;
			}
			let sample_count =
				sample_budget.min(subject.bounds.sample_count()).min(candidate.max_samples);
			let sample_offset = user.contacts.get(&entity).map_or_else(
				|| discovery_sample_cursor.wrapping_add(entity.to_bits() as usize) % samples.len(),
				|contact| {
					usize::try_from(contact.consecutive_failures).unwrap_or(usize::MAX)
						% samples.len()
				},
			);
			let mut visible_point = None;
			let mut visible_head = None;
			for index in 0..sample_count {
				let sample = samples[(sample_offset + index) % samples.len()];
				if !clear_segment(observer, sample.point, &spatial, &fixed_filter) {
					continue;
				}
				if sample.feature.is_head() {
					visible_head = visible_head.or(Some(sample.point));
				} else {
					visible_point = visible_point.or(Some(sample.point));
				}
			}
			let visible_point = visible_point.or(visible_head);
			if let Some(visible_point) = visible_point {
				let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
				match user.contacts.entry(entity) {
					Entry::Vacant(entry) => {
						entry.insert(SpottedContact::new(
							entity,
							transform.translation(),
							velocity,
							visible_point,
							visible_head,
							now,
							policy.respot_interval_secs,
						));
					}
					Entry::Occupied(mut entry) => entry.get_mut().note_success(
						transform.translation(),
						velocity,
						visible_point,
						visible_head,
						now,
						policy.respot_interval_secs,
					),
				}
			} else if let Some(contact) = user.contacts.get_mut(&entity) {
				contact.note_failure(now, policy.respot_interval_secs);
			}
		}

		// Reset only when discovery ran. A Far skip still stretches
		// `next_discovery_at` and still merges due contacts, but must not wipe
		// bake increments or `FAIRNESS_CAP` never fires in a Far-only due set.
		if run_discovery {
			if let Some(lod) = lod.as_deref_mut() {
				lod.skips = 0;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
	use spotting_intelligence::{
		InterestLayers, SpotBounds, SpotDirective, SpottingHint, SpottingObserveLimits,
		SpottingSettings,
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
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
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

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(near)
			.is_some_and(|user| user.contacts.contains_key(&subject)));
		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| !user.contacts.contains_key(&subject)));
		Ok(())
	}

	#[test]
	fn far_broadphase_still_runs_at_fairness_cap() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
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

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| user.contacts.contains_key(&subject)));
		anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
		Ok(())
	}

	#[test]
	fn forget_stale_still_runs_on_far_we_do_not_discover() -> anyhow::Result<()> {
		let mut app = observe_app();
		app.insert_resource(SpottingObserveLimits { max_observers_per_tick: 1 });
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

		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(far)
			.is_some_and(|user| !user.contacts.contains_key(&stale)));
		anyhow::ensure!(app
			.world()
			.get::<SpottingUser>(near)
			.is_some_and(|user| user.contacts.contains_key(&keep)));
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
