//! Bikanwarda Parade, God Beam, and Bray recipes on the reusable
//! [`attacks`] runtime ([#1118](https://github.com/ramate-io/maybraid/issues/1118) /
//! [#1147](https://github.com/ramate-io/maybraid/pull/1147)).
//!
//! This crate does not know about Self. Encounter placement, HCSG, and final
//! VFX stay in their own lanes.

mod cues;
mod ids;
mod parade;
mod params;
mod plugin;
mod presentation;
mod recipes;

pub use cues::{
	cue_plugin, is_bikanwarda_attack, EyeGlowCue, ImpactCue, ParadeExplosionCue, SpatialRumbleCue,
};
pub use ids::{BRAY, GOD_BEAM, PARADE, PARADE_BLAST};
pub use parade::{parade_plugin, ParadeClock};
pub use params::{BikanwardaAttackParams, BrayParams, GodBeamParams, ParadeParams};
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

#[cfg(test)]
mod runtime_tests;
