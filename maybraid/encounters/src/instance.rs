//! Live fixture / runtime body. Not the identity.

use bevy::prelude::*;

use crate::identity::EncounterId;

/// Marks the presented boss (or fixture stand-in) for one [`EncounterId`].
///
/// Progress is **not** stored here. The store keys by `id` so a replacement
/// body or streamed host cannot fork a second progress row.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EncounterInstance {
	pub id: EncounterId,
}

impl EncounterInstance {
	pub fn new(id: EncounterId) -> Self {
		Self { id }
	}
}
