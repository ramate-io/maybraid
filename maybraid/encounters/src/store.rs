//! The one mutable progress owner.

use std::collections::HashMap;

use bevy::prelude::*;
use damage::{Died, Health};

use crate::identity::EncounterId;
use crate::instance::EncounterInstance;
use crate::lifetime::{
	EncounterFinishedReentry, EncounterFinishedReset, EncounterLifetimePolicy, EncounterResetPolicy,
};
use crate::progress::{
	EncounterLifetime, EncounterOutcome, EncounterPhase, EncounterRecord, EncounterSnapshot,
	EncounterSnapshotRecord,
};
use crate::signal::{
	break_precedence, kill_precedes_break, EncounterKill, EncounterResolved, EncounterSignal,
	EncounterSignalKind,
};

/// Sole owner of encounter progress. Other crates read; only
/// [`crate::EncountersPlugin`] writes through resolve systems.
#[derive(Resource, Clone, Debug, Default)]
pub struct EncounterProgressStore {
	records: HashMap<EncounterId, EncounterRecord>,
}

impl EncounterProgressStore {
	pub fn get(&self, id: EncounterId) -> Option<&EncounterRecord> {
		self.records.get(&id)
	}

	pub fn iter(&self) -> impl Iterator<Item = &EncounterRecord> {
		self.records.values()
	}

	pub fn snapshot(&self) -> EncounterSnapshot {
		EncounterSnapshot {
			records: self.records.values().map(EncounterSnapshotRecord::from_record).collect(),
		}
	}

	/// Restore durable flags without live instance bindings.
	pub fn restore(&mut self, snapshot: &EncounterSnapshot) {
		self.records.clear();
		for row in &snapshot.records {
			self.records.insert(row.id, row.into_record());
		}
	}

	/// Scene-side gate: whether a host/fixture should exist for `id` at `elapsed_secs`.
	///
	/// Finished rare-regen is **not** a generation restriction. Callers in
	/// [#1117](https://github.com/ramate-io/maybraid/issues/1117) should consult
	/// this instead of seeding a second probability into HCSG.
	pub fn should_present(
		&self,
		id: EncounterId,
		elapsed_secs: f32,
		policy: &EncounterLifetimePolicy,
	) -> bool {
		let Some(record) = self.records.get(&id) else {
			return true;
		};
		if !record.completed_once {
			return true;
		}
		match policy.finished_reentry {
			EncounterFinishedReentry::StayFinished => false,
			EncounterFinishedReentry::RareTimeSlot { probability, slot_secs } => {
				rare_regen_roll(id, elapsed_secs, slot_secs) < probability.max(0.0)
			}
		}
	}

	pub(crate) fn record_mut(&mut self, id: EncounterId) -> &mut EncounterRecord {
		self.records.entry(id).or_insert_with(|| EncounterRecord::vacant(id))
	}

	pub(crate) fn bind_live(&mut self, id: EncounterId, instance: Entity) {
		let record = self.record_mut(id);
		record.lifetime = EncounterLifetime::InProgress;
		record.phase = EncounterPhase::Combat;
		record.outcome = None;
		record.instance = Some(instance);
		record.sleep_used = false;
		record.last_health_fraction = None;
	}

	pub(crate) fn unbind_instance(&mut self, instance: Entity) -> Option<EncounterId> {
		let id = self.records.values().find(|record| record.instance == Some(instance))?.id;
		if let Some(record) = self.records.get_mut(&id) {
			record.instance = None;
		}
		Some(id)
	}
}

/// Unit interval for the current whole-slot of `elapsed_secs`. Deterministic.
pub fn rare_regen_roll(id: EncounterId, elapsed_secs: f32, slot_secs: f32) -> f32 {
	let slot_secs = if slot_secs.is_finite() && slot_secs > 0.0 { slot_secs } else { 60.0 };
	let elapsed = if elapsed_secs.is_finite() { elapsed_secs.max(0.0) } else { 0.0 };
	let slot = (elapsed / slot_secs).floor() as u64;
	let mut hash = 0xcbf2_9ce4_8422_2325u64;
	for byte in id.as_bytes() {
		hash ^= u64::from(byte);
		hash = hash.wrapping_mul(0x0100_0000_01b3);
	}
	hash ^= slot;
	hash = hash.wrapping_mul(0x0100_0000_01b3);
	((hash >> 11) as f32) / ((1u64 << 53) as f32)
}

pub(crate) fn bind_added_instances(
	added: Query<(Entity, &EncounterInstance), Added<EncounterInstance>>,
	mut store: ResMut<EncounterProgressStore>,
	policy: Res<EncounterLifetimePolicy>,
	time: Res<Time>,
	mut commands: Commands,
) {
	let elapsed = time.elapsed_secs();
	for (entity, instance) in &added {
		if store.should_present(instance.id, elapsed, &policy) {
			store.bind_live(instance.id, entity);
		} else {
			commands.entity(entity).despawn();
		}
	}
}

pub(crate) fn ingest_deaths(
	time: Res<Time>,
	mut died: MessageReader<Died>,
	instances: Query<&EncounterInstance>,
	mut kills: MessageWriter<EncounterResolved>,
	mut store: ResMut<EncounterProgressStore>,
) {
	// Kills are applied immediately with highest precedence so a same-frame
	// chain-break cannot win. Resolve still emits at most one EncounterResolved.
	let now = time.elapsed_secs();
	for death in died.read() {
		let Ok(instance) = instances.get(death.entity) else {
			continue;
		};
		if store
			.get(instance.id)
			.is_some_and(|record| record.instance != Some(death.entity))
		{
			continue;
		}
		apply_outcome(
			&mut store,
			EncounterKill { id: instance.id, instance: death.entity, source: death.source }
				.into_killed(),
			now,
			&mut kills,
		);
	}
}

trait IntoKilled {
	fn into_killed(self) -> PendingOutcome;
}

struct PendingOutcome {
	id: EncounterId,
	outcome: EncounterOutcome,
	instance: Option<Entity>,
	source: Option<Entity>,
}

impl IntoKilled for EncounterKill {
	fn into_killed(self) -> PendingOutcome {
		PendingOutcome {
			id: self.id,
			outcome: EncounterOutcome::Killed,
			instance: Some(self.instance),
			source: self.source,
		}
	}
}

fn apply_outcome(
	store: &mut EncounterProgressStore,
	pending: PendingOutcome,
	now: f32,
	resolved: &mut MessageWriter<EncounterResolved>,
) {
	let record = store.record_mut(pending.id);
	if record.is_finished() {
		return;
	}
	record.lifetime = EncounterLifetime::Finished;
	record.phase = EncounterPhase::Resolved;
	record.outcome = Some(pending.outcome);
	record.instance = pending.instance.or(record.instance);
	record.finished_at_secs = Some(now);
	record.completed_once = true;
	resolved.write(EncounterResolved::new(
		pending.id,
		pending.outcome,
		record.instance,
		pending.source,
	));
}

pub(crate) fn ingest_removed_instances(
	mut removed: RemovedComponents<EncounterInstance>,
	mut store: ResMut<EncounterProgressStore>,
	policy: Res<EncounterLifetimePolicy>,
) {
	for entity in removed.read() {
		let Some(id) = store.unbind_instance(entity) else {
			continue;
		};
		let record = store.record_mut(id);
		if record.is_finished() {
			continue;
		}
		let _ = policy.in_progress_stream_out;
		discard_live(record);
	}
}

pub(crate) fn resolve_signals(
	time: Res<Time>,
	mut signals: MessageReader<EncounterSignal>,
	mut store: ResMut<EncounterProgressStore>,
	policy: Res<EncounterLifetimePolicy>,
	mut resolved: MessageWriter<EncounterResolved>,
	mut healths: Query<&mut Health>,
) {
	let now = time.elapsed_secs();
	let mut ordered: Vec<&EncounterSignal> = signals.read().collect();
	ordered.sort_by_key(|signal| match signal.kind {
		EncounterSignalKind::ChainBroken => break_precedence(),
		_ => kill_precedes_break().saturating_add(2),
	});
	for signal in ordered {
		match signal.kind {
			EncounterSignalKind::ChainBroken => {
				let instance = store.get(signal.id).and_then(|record| record.instance);
				apply_outcome(
					&mut store,
					PendingOutcome {
						id: signal.id,
						outcome: EncounterOutcome::Freed,
						instance,
						source: signal.source,
					},
					now,
					&mut resolved,
				);
			}
			EncounterSignalKind::StreamOut => {
				let record = store.record_mut(signal.id);
				if record.is_finished() {
					record.instance = None;
				} else {
					discard_live(record);
				}
			}
			EncounterSignalKind::PlayerDied | EncounterSignalKind::BodyReplaced => {
				apply_reset(&mut store, policy.on_player_death, &mut healths);
			}
			EncounterSignalKind::SessionReset => {
				apply_reset(&mut store, policy.on_session_reset, &mut healths);
			}
		}
	}
}

fn discard_live(record: &mut EncounterRecord) {
	record.lifetime = EncounterLifetime::Vacant;
	record.phase = EncounterPhase::Inactive;
	record.outcome = None;
	record.instance = None;
	record.finished_at_secs = None;
	record.sleep_used = false;
	record.last_health_fraction = None;
}

fn clear_progress(record: &mut EncounterRecord) {
	discard_live(record);
	record.completed_once = false;
}

fn apply_reset(
	store: &mut EncounterProgressStore,
	policy: EncounterResetPolicy,
	healths: &mut Query<&mut Health>,
) {
	let ids: Vec<EncounterId> = store.records.keys().copied().collect();
	for id in ids {
		let instance = store.get(id).and_then(|record| record.instance);
		let finished = store.get(id).is_some_and(EncounterRecord::is_finished);
		if finished {
			if policy.finished == EncounterFinishedReset::Clear {
				if let Some(record) = store.records.get_mut(&id) {
					clear_progress(record);
				}
			}
			continue;
		}
		let _ = policy.in_progress;
		if let Some(instance) = instance {
			if let Ok(mut health) = healths.get_mut(instance) {
				health.current = health.max;
			}
			store.bind_live(id, instance);
		} else if let Some(record) = store.records.get_mut(&id) {
			discard_live(record);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rare_regen_is_deterministic_per_slot() {
		let id = EncounterId::from_slug("bikanwarda.chained.v0");
		let a = rare_regen_roll(id, 10.0, 60.0);
		let b = rare_regen_roll(id, 59.0, 60.0);
		let c = rare_regen_roll(id, 60.0, 60.0);
		assert_eq!(a, b);
		assert_ne!(a, c);
		assert!((0.0..1.0).contains(&a));
		assert!((0.0..1.0).contains(&c));
	}
}
