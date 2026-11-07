//! Lane consumers for [`crate::cues`] emitted by [`crate::adapter`].
//!
//! | Cue | Provider | Status on `main` |
//! | --- | --- | --- |
//! | [`ParadeExplosionCue`] | `maybraid-vfx` `fiery_explosion` | wired |
//! | [`SpatialRumbleCue`] | `maybraid-spatial-rumble` | wired |
//! | [`BrayAnimationCue`] | `character-motion` `AnimClip::epiphant_bray` | wired ([#1141](https://github.com/ramate-io/maybraid/pull/1141)) |
//! | mouth / [`BikanwardaMouthSocket`] | `character_motion::sample_bray_mouth_emit` | wired |
//! | [`GodBeamVfxCue`] / [`SonicWaveVfxCue`] | `maybraid-vfx` `god_beam` / `sonic_wave` | **PLACEHOLDER** until [#1150](https://github.com/ramate-io/maybraid/pull/1150) (`vfx-driven` feature) |
//! | [`EyeGlowCue`] | `characters::EyeGlowControl` | **pending** [#1152](https://github.com/ramate-io/maybraid/pull/1152) (`eye-glow` feature) |

mod bray_animation;
mod driven_vfx;
mod eye_glow;
mod mouth_emit;
mod parade_vfx;
mod rumble;

use bevy::prelude::*;

pub use crate::lanes::BikanwardaPresentationSystems;
pub use eye_glow::PendingEyeGlow;

/// Maps stable Bikanwarda cues onto landed presentation providers.
///
/// Requires `presentation-consumers` (default). Gameplay and the cue adapter stay
/// available without this plugin — encounters can subscribe manually.
pub struct BikanwardaPresentationConsumersPlugin {
	/// When true, [`driven_vfx`] draws labeled PLACEHOLDER meshes for God Beam / sonic
	/// waves until a driven VFX provider lands ([#1150](https://github.com/ramate-io/maybraid/pull/1150)).
	pub placeholder_driven_vfx: bool,
}

impl Default for BikanwardaPresentationConsumersPlugin {
	fn default() -> Self {
		Self { placeholder_driven_vfx: true }
	}
}

impl Plugin for BikanwardaPresentationConsumersPlugin {
	fn build(&self, app: &mut App) {
		parade_vfx::plugin(app);
		rumble::plugin(app);
		bray_animation::plugin(app);
		mouth_emit::plugin(app);
		eye_glow::plugin(app);
		driven_vfx::plugin(app, self.placeholder_driven_vfx);
	}
}
