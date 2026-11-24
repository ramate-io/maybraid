//! Inputs the store accepts. Presentation teardown is not a reward.

use bevy::prelude::*;

use crate::identity::EncounterId;
use crate::progress::EncounterOutcome;

/// External or ingested request to mutate progress.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterSignal {
	pub id: EncounterId,
	pub kind: EncounterSignalKind,
	pub source: Option<Entity>,
}

impl EncounterSignal {
	pub fn new(id: EncounterId, kind: EncounterSignalKind, source: Option<Entity>) -> Self {
		Self { id, kind, source }
	}

	pub fn chain_broken(id: EncounterId, source: Option<Entity>) -> Self {
		Self::new(id, EncounterSignalKind::ChainBroken, source)
	}

	pub fn player_died(id: EncounterId) -> Self {
		Self::new(id, EncounterSignalKind::PlayerDied, None)
	}

	pub fn body_replaced(id: EncounterId) -> Self {
		Self::new(id, EncounterSignalKind::BodyReplaced, None)
	}

	pub fn session_reset() -> Self {
		Self::new(EncounterId::from_bytes([0; 32]), EncounterSignalKind::SessionReset, None)
	}

	pub fn stream_out(id: EncounterId) -> Self {
		Self::new(id, EncounterSignalKind::StreamOut, None)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EncounterSignalKind {
	/// Chain reached zero durability. Distinct from boss death.
	ChainBroken,
	/// Player/body died. Applies the death reset policy to live fights.
	PlayerDied,
	/// Replacement body. Must not un-finish a completed encounter.
	BodyReplaced,
	/// World session teardown. Broadcast: `id` is ignored.
	SessionReset,
	/// Explicit stream-out. Host despawn is also ingested as this.
	StreamOut,
}

/// Kill ingested from [`damage::Died`] on a bound instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EncounterKill {
	pub id: EncounterId,
	pub instance: Entity,
	pub source: Option<Entity>,
}

/// One committed terminal outcome. Duplicate signals do not emit again.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterResolved {
	pub id: EncounterId,
	pub outcome: EncounterOutcome,
	pub instance: Option<Entity>,
	pub source: Option<Entity>,
}

impl EncounterResolved {
	pub fn new(
		id: EncounterId,
		outcome: EncounterOutcome,
		instance: Option<Entity>,
		source: Option<Entity>,
	) -> Self {
		Self { id, outcome, instance, source }
	}
}

/// Same-frame ordering: kill beats chain-break
/// ([#1121](https://github.com/ramate-io/maybraid/issues/1121)).
pub(crate) fn kill_precedes_break() -> u8 {
	0
}

pub(crate) fn break_precedence() -> u8 {
	1
}
