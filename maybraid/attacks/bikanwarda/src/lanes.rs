//! Presentation lane ordering shared by the cue adapter and consumers.

use bevy::prelude::*;

/// System ordering relative to the attacks runtime.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum BikanwardaPresentationSystems {
	Adapt,
	Consume,
}

pub fn configure_presentation_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(BikanwardaPresentationSystems::Adapt, BikanwardaPresentationSystems::Consume).chain(),
	);
}
