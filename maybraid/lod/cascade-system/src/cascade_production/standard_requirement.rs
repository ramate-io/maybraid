//! [`StandardRequirement`] maps cascade transitions to [`RequirementSignal`] via fixed per-component
//! policies (copied from the two fields each tick).

use bevy::prelude::Component;
use lod_cascade::{Cascade, Chunk};

use super::{CascadePosition, RequirementBuilder, RequirementSignal};

/// [`RequirementBuilder`] that returns configured signals for newly entered vs expired footprints.
///
/// Default matches the [`RequirementBuilder`] trait defaults: **`Visible`** on new,
/// **`Remove`** on expired.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StandardRequirement {
	pub signal_on_new: RequirementSignal,
	pub signal_on_expired: RequirementSignal,
}

impl Default for StandardRequirement {
	fn default() -> Self {
		Self {
			signal_on_new: RequirementSignal::Visible,
			signal_on_expired: RequirementSignal::Remove,
		}
	}
}

impl RequirementBuilder for StandardRequirement {
	fn signal_for_new<D: Component + Clone + Send + Sync + 'static>(
		&self,
		_cascade: &Cascade,
		_position: &CascadePosition<D>,
		_chunk: Chunk,
	) -> RequirementSignal {
		self.signal_on_new
	}

	fn signal_for_expired<D: Component + Clone + Send + Sync + 'static>(
		&self,
		_cascade: &Cascade,
		_position: &CascadePosition<D>,
		_chunk: Chunk,
	) -> RequirementSignal {
		self.signal_on_expired
	}
}
