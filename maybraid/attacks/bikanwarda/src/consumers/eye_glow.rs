//! [`EyeGlowCue`] → `characters::EyeGlowControl` ([#1152](https://github.com/ramate-io/maybraid/pull/1152)).
//!
//! Until that PR lands, this consumer only tracks owners that should glow so tests
//! and encounter scripts can query [`PendingEyeGlow`].

use bevy::prelude::*;

use crate::EyeGlowCue;
use crate::consumers::BikanwardaPresentationSystems;

/// Owners with an active God Beam windup glow request when `eye-glow` is off.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingEyeGlow {
	pub owners: std::collections::HashSet<Entity>,
}

pub fn plugin(app: &mut App) {
	app.init_resource::<PendingEyeGlow>();
	app.add_systems(Update, consume_eye_glow_cues.in_set(BikanwardaPresentationSystems::Consume));
}

fn consume_eye_glow_cues(mut cues: MessageReader<EyeGlowCue>, mut pending: ResMut<PendingEyeGlow>) {
	for cue in cues.read() {
		if cue.active {
			pending.owners.insert(cue.owner);
		} else {
			pending.owners.remove(&cue.owner);
		}
	}
}
