//! Live progress and terminal outcomes. Not reconstructed from health.

use crate::identity::EncounterId;

/// Whether this identity currently has a live fight, a finished flag, or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EncounterLifetime {
	/// Never presented, or in-progress state discarded (stream-out / load-fresh).
	Vacant,
	InProgress,
	Finished,
}

/// Explicit live phase. Sleep is a later driver; do not infer this from HP.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EncounterPhase {
	Inactive,
	Combat,
	/// Sleep/recovery. The sleep driver ([#1120](https://github.com/ramate-io/maybraid/issues/1120))
	/// owns transitions into and out of this variant.
	Sleep(EncounterSleep),
	Waking,
	Resolved,
}

/// Sleep session recorded on the progress owner, not on generated data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncounterSleep {
	pub elapsed_secs: f32,
	pub health_at_enter: f32,
}

impl EncounterSleep {
	pub fn new(health_at_enter: f32) -> Self {
		Self {
			elapsed_secs: 0.0,
			health_at_enter: if health_at_enter.is_finite() {
				health_at_enter.max(0.0)
			} else {
				0.0
			},
		}
	}
}

/// Terminal encounter result. One identity stores at most one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EncounterOutcome {
	/// Boss died. Bikanwarda maps this to Enraged.
	Killed,
	/// Chain broke / she was freed. Bikanwarda maps this to Ward.
	Freed,
}

/// One identity's durable progress. The store is the sole owner of this value.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterRecord {
	pub id: EncounterId,
	pub lifetime: EncounterLifetime,
	pub phase: EncounterPhase,
	pub outcome: Option<EncounterOutcome>,
	/// Live body when presented. Presentation teardown clears this without
	/// writing an outcome.
	pub instance: Option<bevy::prelude::Entity>,
	pub finished_at_secs: Option<f32>,
	/// True after the first terminal outcome. Rare regen may start a new
	/// occurrence without forgetting that the identity has completed once.
	pub completed_once: bool,
}

impl EncounterRecord {
	pub fn vacant(id: EncounterId) -> Self {
		Self {
			id,
			lifetime: EncounterLifetime::Vacant,
			phase: EncounterPhase::Inactive,
			outcome: None,
			instance: None,
			finished_at_secs: None,
			completed_once: false,
		}
	}

	/// This occurrence has a terminal outcome.
	pub fn is_finished(&self) -> bool {
		self.lifetime == EncounterLifetime::Finished || self.outcome.is_some()
	}
}

/// Disk-agnostic snapshot. Does not include live entity ids.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterSnapshot {
	pub records: Vec<EncounterSnapshotRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncounterSnapshotRecord {
	pub id: EncounterId,
	pub lifetime: EncounterLifetime,
	pub phase: EncounterPhase,
	pub outcome: Option<EncounterOutcome>,
	pub finished_at_secs: Option<f32>,
	pub completed_once: bool,
}

impl EncounterSnapshotRecord {
	pub fn from_record(record: &EncounterRecord) -> Self {
		Self {
			id: record.id,
			lifetime: record.lifetime,
			phase: record.phase,
			outcome: record.outcome,
			finished_at_secs: record.finished_at_secs,
			completed_once: record.completed_once,
		}
	}

	pub fn into_record(self) -> EncounterRecord {
		EncounterRecord {
			id: self.id,
			lifetime: self.lifetime,
			phase: self.phase,
			outcome: self.outcome,
			instance: None,
			finished_at_secs: self.finished_at_secs,
			completed_once: self.completed_once,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn snapshot_drops_live_instance_ids() {
		let id = EncounterId::from_slug("snap");
		let mut record = EncounterRecord::vacant(id);
		record.lifetime = EncounterLifetime::Finished;
		record.outcome = Some(EncounterOutcome::Killed);
		record.completed_once = true;
		record.instance = Some(bevy::prelude::Entity::from_bits(7));
		let snap = EncounterSnapshotRecord::from_record(&record);
		assert_eq!(snap.outcome, Some(EncounterOutcome::Killed));
		let restored = snap.into_record();
		assert!(restored.instance.is_none());
		assert!(restored.completed_once);
	}
}
