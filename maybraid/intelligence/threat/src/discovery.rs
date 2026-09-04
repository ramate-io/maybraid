use std::collections::HashSet;

use bevy::prelude::*;
use spotting_intelligence::{SpottingHintSource, SpottingUser};

use crate::{
	Affiliations, ThreatIntelligenceUser, ThreatKnowledge, ThreatObservation, ThreatRegistry,
	ThreatSource, ThreatSubject,
};

type ThreatEntity<'a> = (Entity, &'a ThreatSubject, &'a Affiliations, &'a GlobalTransform);
type ChangedThreatEntity = Or<(
	Added<ThreatSubject>,
	Changed<ThreatSubject>,
	Changed<Affiliations>,
	Changed<GlobalTransform>,
)>;
type ThreatRecipients<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static ThreatSubject,
		&'static GlobalTransform,
		Ref<'static, Affiliations>,
		&'static mut ThreatIntelligenceUser,
		&'static mut ThreatKnowledge,
	),
>;

/// Mirrors live semantic threat subjects into the local Gimme index.
pub fn sync_threat_registry(
	changed: Query<ThreatEntity<'_>, ChangedThreatEntity>,
	current: Query<ThreatEntity<'_>>,
	mut removed_subjects: RemovedComponents<ThreatSubject>,
	mut removed_affiliations: RemovedComponents<Affiliations>,
	mut registry: ResMut<ThreatRegistry>,
) {
	for (entity, subject, affiliations, transform) in &changed {
		upsert(&mut registry, entity, *subject, affiliations, transform.translation());
	}
	let mut removed: Vec<_> = removed_subjects.read().chain(removed_affiliations.read()).collect();
	removed.sort();
	removed.dedup();
	for entity in removed {
		if let Ok((entity, subject, affiliations, transform)) = current.get(entity) {
			upsert(&mut registry, entity, *subject, affiliations, transform.translation());
		} else {
			registry.remove_entity(entity);
		}
	}
}

fn upsert(
	registry: &mut ThreatRegistry,
	entity: Entity,
	subject: ThreatSubject,
	affiliations: &Affiliations,
	position: Vec3,
) {
	if let Err(error) = registry.upsert(entity, subject, affiliations, position) {
		warn!("failed to index threat subject {entity}: {error}");
	}
}

/// Applies directed non-spatial findings through the same affiliation gate as scans.
pub fn ingest_threat_observations(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut observations: MessageReader<ThreatObservation>,
	mut recipients: Query<(&Affiliations, &ThreatIntelligenceUser, &mut ThreatKnowledge)>,
) {
	let now = time.elapsed_secs();
	for observation in observations.read() {
		let Some(record) = registry.get(observation.subject) else {
			continue;
		};
		if record.entity == observation.recipient {
			continue;
		}
		let Ok((affiliations, user, mut knowledge)) = recipients.get_mut(observation.recipient)
		else {
			continue;
		};
		knowledge.observe(
			record,
			affiliations,
			observation.source,
			observation.confidence,
			now,
			user.policy.threat_threshold,
		);
	}
}

/// Periodically discovers nearby hostile affiliations and maintains retained knowledge.
pub fn discover_threats(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	mut recipients: ThreatRecipients,
) {
	let now = time.elapsed_secs();
	for (entity, identity, transform, affiliations, mut user, mut knowledge) in &mut recipients {
		knowledge.reconcile_registry(&registry);
		knowledge.maintain(&affiliations, user.policy, now);
		if affiliations.is_changed() {
			user.next_scan_at = 0.0;
		}
		if now < user.next_scan_at {
			continue;
		}
		let candidates = registry.local(transform.translation(), user.policy.radius);
		let count = candidates.len();
		let budget = user.policy.candidates_per_scan.min(count);
		for offset in 0..budget {
			let record = &candidates[(user.sample_cursor + offset) % count];
			if record.entity == entity || record.id == identity.id {
				continue;
			}
			knowledge.observe(
				record,
				&affiliations,
				ThreatSource::LOCAL_SCAN,
				1.0,
				now,
				user.policy.threat_threshold,
			);
		}
		user.sample_cursor = user.sample_cursor.wrapping_add(budget.max(1));
		let interval = if knowledge.len() >= user.policy.desired_threats {
			user.policy.retained_scan_interval_secs
		} else {
			user.policy.scan_interval_secs
		};
		user.next_scan_at = now + staggered_interval(interval, entity);
		knowledge.maintain(&affiliations, user.policy, now);
	}
}

/// Reconciles retained threats into one independently-owned spotting hint source.
pub fn export_threat_spotting_hints(mut recipients: Query<(&ThreatKnowledge, &mut SpottingUser)>) {
	for (knowledge, mut spotting) in &mut recipients {
		let active: HashSet<Entity> = knowledge.iter().filter_map(|known| known.entity).collect();
		let retired: Vec<_> = spotting
			.hints
			.iter()
			.filter_map(|(entity, hint)| {
				(hint.has_source(SpottingHintSource::THREAT) && !active.contains(entity))
					.then_some(*entity)
			})
			.collect();
		for entity in retired {
			spotting.remove_hint_source(entity, SpottingHintSource::THREAT);
		}
		for known in knowledge.iter() {
			let Some(entity) = known.entity else {
				continue;
			};
			let priority = (known.threat_weight * known.confidence * 4.0)
				.round()
				.clamp(1.0, i32::MAX as f32) as i32;
			spotting.hint_from(entity, SpottingHintSource::THREAT, priority);
		}
	}
}

fn staggered_interval(interval: f32, entity: Entity) -> f32 {
	let jitter = (entity.to_bits() % 1_001) as f32 / 1_000.0;
	interval.max(0.05) * (0.8 + jitter * 0.4)
}
