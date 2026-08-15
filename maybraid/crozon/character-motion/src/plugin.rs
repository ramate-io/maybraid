//! Mailbox systems. Elevation is registered by the app with a concrete probe.

use bevy::prelude::*;

use crate::mailbox::{prepare_anim_mailbox, tick_anim_mailbox};

/// Per-frame articulation sets. Recipes schedule structural pose **before**
/// [`Self::Anim`]. Playgrounds schedule elevation after physics / locomotion.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterMotionSystems {
	/// Prepare + tick the clip mailbox.
	Anim,
	/// `apply_terrain_pitch::<P>` — add this system yourself with a probe.
	Elevation,
}

/// Clip mailbox. Does not register elevation (that is generic over a probe).
///
/// Apps that also run `crozon-characters` should order
/// [`CharacterMotionSystems::Anim`] after `CharacterHostSystems::Pose`.
pub struct CharacterMotionPlugin;

impl Plugin for CharacterMotionPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			CharacterMotionSystems::Elevation.after(CharacterMotionSystems::Anim),
		);
		app.add_systems(
			Update,
			(prepare_anim_mailbox, tick_anim_mailbox.after(prepare_anim_mailbox))
				.in_set(CharacterMotionSystems::Anim),
		);
	}
}
