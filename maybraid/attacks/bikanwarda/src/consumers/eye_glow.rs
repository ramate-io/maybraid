//! [`EyeGlowCue`] → `characters::EyeGlowControl` ([#1152](https://github.com/ramate-io/maybraid/pull/1152)).
//!
//! Until that PR lands, this consumer only tracks owners that should glow so tests
//! and encounter scripts can query [`PendingEyeGlow`].

use bevy::prelude::*;

use crate::consumers::BikanwardaPresentationSystems;
use crate::EyeGlowCue;

/// Owners with an active God Beam windup glow request when `eye-glow` is off.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingEyeGlow {
	pub owners: std::collections::HashSet<Entity>,
}

pub fn plugin(app: &mut App) {
	app.init_resource::<PendingEyeGlow>();
	app.add_systems(
		Update,
		consume_eye_glow_cues.in_set(BikanwardaPresentationSystems::Consume),
	);
}

fn consume_eye_glow_cues(
	mut cues: MessageReader<EyeGlowCue>,
	mut pending: ResMut<PendingEyeGlow>,
) {
	for cue in cues.read() {
		if cue.active {
			pending.owners.insert(cue.owner);
		} else {
			pending.owners.remove(&cue.owner);
		}
	}
}

#[cfg(feature = "eye-glow")]
mod wired {
	use super::*;
	use characters::EyeGlowControl;

	pub fn plugin(app: &mut App) {
		app.add_systems(
			Update,
			apply_eye_glow_control.in_set(BikanwardaPresentationSystems::Consume),
		);
	}

	fn apply_eye_glow_control(
		mut cues: MessageReader<EyeGlowCue>,
		mut owners: Query<&mut EyeGlowControl>,
		mut commands: Commands,
	) {
		for cue in cues.read() {
			if let Ok(mut control) = owners.get_mut(cue.owner) {
				if cue.active {
					control.start_god_beam_warning();
				} else {
					control.cancel();
				}
				continue;
			}
			if cue.active {
				commands.entity(cue.owner).insert(EyeGlowControl::god_beam_warning());
			}
		}
	}
}

#[cfg(feature = "eye-glow")]
pub use wired::plugin as eye_glow_wired_plugin;
