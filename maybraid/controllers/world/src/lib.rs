//! World controller: pause / live-play gate for character input.

pub mod enabled;

pub use enabled::WorldGameplayEnabled;

use bevy::prelude::*;

/// Session-level capability resolution (gameplay / physics / streaming / …).
///
/// Runs after pause intents flip [`bevy::state::state::NextState`] and before
/// the world motor filter consumes [`WorldGameplayEnabled`]. Actor locomotion
/// mode stays on `TanzUserSystems::Toggle` (before pad bind).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActivityPolicySystems;

pub struct WorldControllerPlugin;

impl Plugin for WorldControllerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldGameplayEnabled>();
	}
}
