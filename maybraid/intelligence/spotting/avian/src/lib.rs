//! Avian broadphase discovery and fixed-geometry line-of-sight for
//! `spotting-intelligence`.

mod candidate;
mod discover;
mod los;
mod observe;
mod probe;

use bevy::prelude::*;
use intelligence_lod::IntelligencePriority;
use spotting_intelligence::{SpottingObserveLimits, SpottingSystems};

pub use los::{RAY_ORIGIN_SKIP, clear_segment};
#[doc(hidden)]
pub use observe::SpottingFrameState;
pub use observe::{discover_spotting, prepare_spotting, refresh_spotting};

/// Installs Avian-backed observation in [`SpottingSystems::Observe`].
pub struct SpottingAvianPlugin;

impl Plugin for SpottingAvianPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<IntelligencePriority>()
			.init_resource::<SpottingObserveLimits>()
			.init_resource::<SpottingFrameState>()
			.configure_sets(
				Update,
				(SpottingSystems::Prepare, SpottingSystems::Discover, SpottingSystems::Refresh)
					.chain()
					.in_set(SpottingSystems::Observe),
			)
			.add_systems(Update, prepare_spotting.in_set(SpottingSystems::Prepare))
			.add_systems(Update, discover_spotting.in_set(SpottingSystems::Discover))
			.add_systems(Update, refresh_spotting.in_set(SpottingSystems::Refresh));
	}
}
