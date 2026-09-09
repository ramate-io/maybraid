use std::collections::HashSet;

use bevy::prelude::*;
use intelligence_lod::{due_by_rank, IntelligenceBand, IntelligenceLod, IntelligencePriority};
use spotting_intelligence::{SpottingHintSource, SpottingUser};

use crate::{
	Affiliations, ThreatDiscoverLimits, ThreatIntelligenceUser, ThreatKnowledge, ThreatObservation,
	ThreatRegistry, ThreatSource, ThreatSubject,
};

type ThreatEntity<'a> = (
	Entity,
	Ref<'a, ThreatSubject>,
	Ref<'a, Affiliations>,
	&'a GlobalTransform,
	Option<&'a IntelligenceLod>,
);
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
		Option<&'static mut IntelligenceLod>,
	),
>;

/// Mirrors live semantic threat subjects into the local Gimme index.
///
/// Identity changes always upsert. Translation-only updates are quantized by
/// [`IntelligenceLod`] (missing = Near). Walking Mid/Far plants do not rebuild
/// the index every frame.
pub fn sync_threat_registry(
	changed: Query<ThreatEntity<'_>, ChangedThreatEntity>,
	current: Query<ThreatEntity<'_>>,
	mut removed_subjects: RemovedComponents<ThreatSubject>,
	mut removed_affiliations: RemovedComponents<Affiliations>,
	mut registry: ResMut<ThreatRegistry>,
) {
	for (entity, subject, affiliations, transform, lod) in &changed {
		let identity_changed = subject.is_changed() || affiliations.is_changed();
		let position = transform.translation();
		if !identity_changed {
			let band = IntelligenceLod::band_or_near(lod);
			if let Some(existing) = registry.get_entity(entity) {
				if position.distance(existing.position) < move_quantum(band) {
					continue;
				}
			}
		}
		upsert(&mut registry, entity, *subject, &affiliations, position);
	}
	let mut removed: Vec<_> = removed_subjects.read().chain(removed_affiliations.read()).collect();
	removed.sort();
	removed.dedup();
	for entity in removed {
		if let Ok((entity, subject, affiliations, transform, _)) = current.get(entity) {
			upsert(&mut registry, entity, *subject, &affiliations, transform.translation());
		} else {
			registry.remove_entity(entity);
		}
	}
}

fn move_quantum(band: IntelligenceBand) -> f32 {
	match band {
		IntelligenceBand::Near => 1.0,
		IntelligenceBand::Mid => 4.0,
		IntelligenceBand::Far => 16.0,
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

/// Forget on a staggered clock, then drain due scans in shared viewer order.
///
/// Far skips the scan body unless a fairness reserve is due. `skips` reset when
/// a scan actually ran. Missing lod is Near (local player).
pub fn discover_threats(
	time: Res<Time>,
	registry: Res<ThreatRegistry>,
	priority: Res<IntelligencePriority>,
	limits: Res<ThreatDiscoverLimits>,
	mut recipients: ThreatRecipients,
) {
	let now = time.elapsed_secs();
	for (entity, _, _, affiliations, mut user, mut knowledge, _) in &mut recipients {
		if affiliations.is_changed() {
			user.next_scan_at = 0.0;
		}
		forget_if_due(entity, now, &affiliations, &mut user, &mut knowledge, &registry);
	}

	let mut due: Vec<Entity> = recipients
		.iter_mut()
		.filter_map(|(entity, _, _, _, user, _, _)| (now >= user.next_scan_at).then_some(entity))
		.collect();
	due_by_rank(&mut due, &priority);
	let fair = due.iter().copied().find(|entity| {
		recipients.get_mut(*entity).is_ok_and(|(_, _, _, _, _, _, lod)| {
			lod.as_deref().is_some_and(|lod| {
				lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
			})
		})
	});

	let mut remaining = limits.max_scans_per_tick;
	for entity in due {
		if remaining == 0 {
			break;
		}
		let Ok((entity, identity, transform, affiliations, mut user, mut knowledge, mut lod)) =
			recipients.get_mut(entity)
		else {
			continue;
		};
		let band = IntelligenceLod::band_or_near(lod.as_deref());
		if band == IntelligenceBand::Far && Some(entity) != fair {
			continue;
		}
		remaining -= 1;
		knowledge.reconcile_registry(&registry);
		knowledge.maintain(&affiliations, user.policy, now);
		user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
		let candidates = registry.local(transform.translation(), user.policy.radius);
		let count = candidates.len();
		let budget = band.scale_count(user.policy.candidates_per_scan).min(count);
		let mut taken = 0;
		let mut offset = 0;
		while taken < budget && offset < count {
			let record = &candidates[(user.sample_cursor + offset) % count];
			offset += 1;
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
			taken += 1;
		}
		user.sample_cursor = user.sample_cursor.wrapping_add(offset.max(1));
		let interval = if knowledge.len() >= user.policy.desired_threats {
			user.policy.retained_scan_interval_secs
		} else {
			user.policy.scan_interval_secs
		};
		user.next_scan_at = now + staggered_interval(interval, entity, 0) * band.interval_scale();
		if let Some(lod) = lod.as_deref_mut() {
			lod.skips = 0;
		}
	}
}

const FORGET_INTERVAL: f32 = 2.0;

fn forget_if_due(
	entity: Entity,
	now: f32,
	affiliations: &Affiliations,
	user: &mut ThreatIntelligenceUser,
	knowledge: &mut ThreatKnowledge,
	registry: &ThreatRegistry,
) {
	if now < user.next_forget_at {
		return;
	}
	knowledge.reconcile_registry(registry);
	knowledge.maintain(affiliations, user.policy, now);
	user.next_forget_at = now + staggered_interval(FORGET_INTERVAL, entity, 2);
}

/// Reconciles retained threats into one independently-owned spotting hint source.
///
/// Only runs when knowledge changed this frame (scan, forget, or inbox).
pub fn export_threat_spotting_hints(
	mut recipients: Query<(&ThreatKnowledge, &mut SpottingUser), Changed<ThreatKnowledge>>,
) {
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

fn staggered_interval(interval: f32, entity: Entity, salt: u64) -> f32 {
	let bits = entity.to_bits().wrapping_add(salt.wrapping_mul(0x9e37_79b9));
	let jitter = (bits % 1_001) as f32 / 1_000.0;
	interval.max(0.05) * (0.8 + jitter * 0.4)
}
