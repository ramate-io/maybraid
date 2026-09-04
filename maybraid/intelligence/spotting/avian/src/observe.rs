use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use avian3d::prelude::{Collider, LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{
	allocate_sample_budget, apply_candidate_budget, rank_candidates, SpotCandidate,
	SpotContactView, SpotDirective, SpotSubject, SpottedContact, SpottingUser,
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

/// Discover, rank, and visibility-probe subjects for every spotting user.
pub fn observe_spotting(
	spatial: SpatialQuery,
	time: Res<Time>,
	mut spotters: Query<(Entity, &mut SpottingUser)>,
	subjects: Query<(Entity, &SpotSubject, Option<&LinearVelocity>)>,
	parents: Query<&ChildOf>,
	transforms: TransformHelper,
) {
	let now = time.elapsed_secs();
	let animated_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Animated);
	let fixed_filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);

	for (spotter_entity, mut user) in &mut spotters {
		user.forget_stale(now);
		user.contacts.retain(|entity, _| subjects.get(*entity).is_ok());
		let Ok(spotter_transform) = transforms.compute_global_transform(spotter_entity) else {
			continue;
		};
		let observer =
			spotter_transform.translation() + spotter_transform.rotation() * user.eye_offset;
		let discovery_due = now >= user.next_discovery_at;
		let discovery_sample_cursor = if discovery_due { user.advance_sample_cursor() } else { 0 };
		let mut candidates = BTreeMap::new();

		if discovery_due {
			for &directive in &user.directives {
				if directive.desired_count == 0
					|| directive_satisfied(directive, now, observer, &user, &subjects, &transforms)
				{
					continue;
				}
				let range = directive.range.max(0.0);
				if range == 0.0 || !range.is_finite() {
					continue;
				}
				let sphere = Collider::sphere(range);
				for entity in
					spatial.shape_intersections(&sphere, observer, Quat::IDENTITY, &animated_filter)
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
						&mut candidates,
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

			let next_interval = user
				.directives
				.iter()
				.map(|directive| directive.discovery_interval_secs.max(0.0))
				.reduce(f32::min)
				.unwrap_or(0.25);
			user.next_discovery_at = now + next_interval;
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
		apply_candidate_budget(&mut ranked, user.settings.candidate_budget);
		let grants = allocate_sample_budget(
			&ranked,
			user.settings.candidate_budget,
			user.settings.vision_samples,
		);

		for (candidate, sample_budget) in ranked.into_iter().zip(grants) {
			if sample_budget == 0 {
				continue;
			}
			let Some(policy) = candidates.get(&candidate.subject) else {
				continue;
			};
			// Resolve transform and velocity at probe time rather than relying
			// on broadphase snapshots or remembered positions.
			let Ok((entity, subject, velocity)) = subjects.get(candidate.subject) else {
				continue;
			};
			let Ok(transform) = transforms.compute_global_transform(entity) else {
				continue;
			};
			let samples = subject.bounds.samples(observer, transform.translation());
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
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
}
