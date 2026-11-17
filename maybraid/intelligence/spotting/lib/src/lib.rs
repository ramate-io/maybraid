//! Physics-independent spotting policy, semantic subjects, and contact memory.
//!
//! Add [`SpotSubject`] to discoverable entities and [`SpottingUser`] to
//! observers. A backend such as `spotting-intelligence-avian` performs
//! broadphase discovery and visibility probes.

mod bounds;
mod candidate;
mod contact;
mod directive;
mod layers;
mod subject;
mod user;

use bevy::prelude::*;

pub use bounds::{SpotBounds, SpotFeature, SpotSample};
pub use candidate::SpotCandidate;
pub use contact::SpottedContact;
pub use directive::{SpotContactView, SpotDirective};
pub use layers::InterestLayers;
pub use subject::SpotSubject;
pub use user::{SpottingHint, SpottingHintSource, SpottingSettings, SpottingUser};

/// Shared schedule location for spotting backends.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpottingSystems {
	/// Parent set retained for application ordering and optional whole-pipeline gates.
	Observe,
	/// Clear this Update's service ledger and expire invalid contacts.
	Prepare,
	/// Broadphase and explicit-hint discovery; applications own its cadence.
	Discover,
	/// Bounded existing-contact refresh; applications own its cadence.
	Refresh,
}

/// Independent observer budgets for discovery and additional retained-contact refresh.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpottingObserveLimits {
	/// Maximum discovery observers, including their due retained contacts.
	pub max_observers_per_tick: usize,
	/// Maximum additional observers that may refresh retained contacts without discovery.
	pub max_retained_observers_per_tick: usize,
	/// Maximum allocated visibility samples across discovery observers in one Update.
	pub max_discovery_samples_per_tick: usize,
	/// Independent maximum allocated samples across retained observers in one Update.
	pub max_retained_samples_per_tick: usize,
}

impl Default for SpottingObserveLimits {
	fn default() -> Self {
		Self {
			max_observers_per_tick: 8,
			max_retained_observers_per_tick: 8,
			max_discovery_samples_per_tick: 384,
			max_retained_samples_per_tick: 384,
		}
	}
}
