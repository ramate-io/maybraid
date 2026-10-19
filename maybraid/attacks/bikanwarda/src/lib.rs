//! Bikanwarda Parade, God Beam, and Bray recipes on the reusable
//! [`attacks`] runtime ([#1118](https://github.com/ramate-io/maybraid/issues/1118) /
//! [#1147](https://github.com/ramate-io/maybraid/pull/1147)).
//!
//! This crate does not know about Self. Encounter placement, HCSG, and final
//! VFX stay in their own lanes.

mod adapter;
#[cfg(feature = "presentation-consumers")]
mod consumers;
mod cues;
mod ids;
mod lanes;
mod parade;
mod params;
mod plugin;
mod presentation;
mod recipes;

pub use adapter::presentation_adapter_plugin;
pub use cues::{
	cue_plugin, is_bikanwarda_attack, BrayAnimationCue, BrayAnimationPhase, EyeGlowCue,
	GodBeamVfxCue, ImpactCue, ParadeExplosionCue, SonicWaveVfxCue, SonicWaveVfxKind,
	SpatialRumbleCue, SpatialRumblePreset,
};
pub use ids::{BRAY, GOD_BEAM, PARADE, PARADE_BLAST};
pub use parade::{parade_plugin, ParadeClock};
pub use params::{BikanwardaAttackParams, BrayParams, GodBeamParams, ParadeParams};
#[cfg(feature = "presentation-consumers")]
pub use consumers::BikanwardaPresentationConsumersPlugin;
#[cfg(feature = "presentation-consumers")]
pub use consumers::PendingEyeGlow;
pub use lanes::BikanwardaPresentationSystems;
pub use plugin::BikanwardaAttacksPlugin;
pub use presentation::{
	placeholder_presentation_plugin, BikanwardaEyeSocket, BikanwardaHeadSocket,
	BikanwardaMouthSocket,
};
pub use recipes::{
	bray_definition, god_beam_definition, parade_blast_definition, parade_definition,
	register_bikanwarda_attacks,
};

/// Documented placeholder GLB sockets. See [`ASSETS.md`](../ASSETS.md).
pub const ASSETS_README: &str = include_str!("../ASSETS.md");

#[cfg(all(test, feature = "presentation-consumers"))]
mod consumer_tests;

#[cfg(test)]
mod runtime_tests;
