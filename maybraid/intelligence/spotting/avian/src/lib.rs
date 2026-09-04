//! Avian broadphase discovery and fixed-geometry line-of-sight for
//! `spotting-intelligence`.

mod los;
mod observe;

use bevy::prelude::*;
use spotting_intelligence::SpottingSystems;

pub use los::{clear_segment, RAY_ORIGIN_SKIP};
pub use observe::observe_spotting;

/// Installs Avian-backed observation in [`SpottingSystems::Observe`].
pub struct SpottingAvianPlugin;

impl Plugin for SpottingAvianPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, observe_spotting.in_set(SpottingSystems::Observe));
	}
}
