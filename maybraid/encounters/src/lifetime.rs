//! Stream-out, death, session reset, and save policy.
//!
//! These are configuration, not hidden combat rules. Liam's Oct 12 notes on
//! [#1121](https://github.com/ramate-io/maybraid/issues/1121) are
//! [`Self::bikanwarda`]. Open questions stay labeled rather than closed.

/// What happens to an in-progress live fight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterLiveReset {
	/// Drop live phase/health ownership. Next present is a new fight.
	LoadFresh,
}

/// What happens to a finished identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterFinishedReset {
	/// Keep the terminal outcome. Body replace must not un-finish.
	Retain,
	/// Clear the finished flag (load as a first encounter).
	Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterResetPolicy {
	pub in_progress: EncounterLiveReset,
	pub finished: EncounterFinishedReset,
}

/// How a finished encounter may come back after stream-out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EncounterFinishedReentry {
	/// Do not recreate a completed encounter.
	StayFinished,
	/// Rare regen, selected by whole-second time slots. Scene-side, not a
	/// generation restriction ([#1121](https://github.com/ramate-io/maybraid/issues/1121)).
	RareTimeSlot { probability: f32, slot_secs: f32 },
}

/// In-progress stream-out: first encounter does not persist live HP/phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterStreamPolicy {
	DiscardLive,
}

/// Process-restart persistence. The repo has no world-encounter save file yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterDiskPersistence {
	/// In-memory snapshot only. Disk write is an explicit follow-up.
	SessionOnly,
}

/// Pause / menu gate. World copies `WorldGameplayEnabled` onto this.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterTimeEnabled(pub bool);

impl Default for EncounterTimeEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Authored lifetime / re-entry / save policy. One resource for the store.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq)]
pub struct EncounterLifetimePolicy {
	pub in_progress_stream_out: EncounterStreamPolicy,
	pub finished_reentry: EncounterFinishedReentry,
	pub on_player_death: EncounterResetPolicy,
	pub on_session_reset: EncounterResetPolicy,
	pub disk: EncounterDiskPersistence,
}

impl Default for EncounterLifetimePolicy {
	fn default() -> Self {
		Self {
			in_progress_stream_out: EncounterStreamPolicy::DiscardLive,
			finished_reentry: EncounterFinishedReentry::StayFinished,
			on_player_death: EncounterResetPolicy {
				in_progress: EncounterLiveReset::LoadFresh,
				finished: EncounterFinishedReset::Retain,
			},
			on_session_reset: EncounterResetPolicy {
				in_progress: EncounterLiveReset::LoadFresh,
				finished: EncounterFinishedReset::Retain,
			},
			disk: EncounterDiskPersistence::SessionOnly,
		}
	}
}

impl EncounterLifetimePolicy {
	/// Liam Oct 12: in-progress stream-out discards; finished is saved for the
	/// session and may rare-regen on ~5% one-minute slots; death/body replace
	/// load a fresh live fight without clearing a finished flag.
	///
	/// Open: whether `WorldPlaySessionEnded` should [`EncounterFinishedReset::Clear`]
	/// while disk persistence is still [`EncounterDiskPersistence::SessionOnly`].
	/// This default retains finished so body/session teardown cannot accidentally
	/// un-award a completed encounter.
	pub fn bikanwarda() -> Self {
		Self {
			finished_reentry: EncounterFinishedReentry::RareTimeSlot {
				probability: 0.05,
				slot_secs: 60.0,
			},
			..Self::default()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bikanwarda_policy_keeps_finished_across_death() {
		let policy = EncounterLifetimePolicy::bikanwarda();
		assert_eq!(policy.on_player_death.finished, EncounterFinishedReset::Retain);
		assert_eq!(policy.on_session_reset.finished, EncounterFinishedReset::Retain);
		assert_eq!(policy.disk, EncounterDiskPersistence::SessionOnly);
		assert!(matches!(
			policy.finished_reentry,
			EncounterFinishedReentry::RareTimeSlot { probability, slot_secs }
				if (probability - 0.05).abs() < f32::EPSILON && (slot_secs - 60.0).abs() < f32::EPSILON
		));
	}
}
