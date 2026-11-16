//! Stable encounter identity and the sole mutable progress owner.
//!
//! Generated HCSG values must not decide completion or rewards. This crate
//! keys progress by [`EncounterId`] and mutates it only in
//! [`EncountersPlugin`]. Fixture instances are the development host until
//! [#1117](https://github.com/ramate-io/maybraid/issues/1117).

mod identity;
mod instance;
mod lifetime;
mod plugin;
mod progress;
mod signal;
mod store;

#[cfg(any(test, feature = "test-support"))]
pub mod fixtures;

use bevy::prelude::*;

pub use identity::EncounterId;
pub use instance::EncounterInstance;
pub use lifetime::{
	EncounterDiskPersistence, EncounterFinishedReentry, EncounterFinishedReset,
	EncounterLifetimePolicy, EncounterLiveReset, EncounterResetPolicy, EncounterStreamPolicy,
	EncounterTimeEnabled,
};
pub use plugin::EncountersPlugin;
pub use progress::{
	EncounterLifetime, EncounterOutcome, EncounterPhase, EncounterRecord, EncounterSleep,
	EncounterSnapshot, EncounterSnapshotRecord,
};
pub use signal::{EncounterResolved, EncounterSignal, EncounterSignalKind};
pub use store::{rare_regen_roll, EncounterProgressStore};

/// Schedule sets content crates order against. Award unlocks after [`Self::Resolve`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EncounterSystems {
	Ingest,
	Resolve,
}

#[cfg(test)]
mod tests;
