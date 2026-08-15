//! LOD band policy, baked onto the host (defaults) and each [`lod::LodLevelRoot`] child.
//!
//! Systems read the **shown** level child. Do not rebuild a level to flip a bool.

use bevy::prelude::*;

/// Write bone pose from the mailbox (`Animation::apply_for`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimateBones;

/// Write armature root-motion from the mailbox (`Animation::effects_for`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimateEffects;

/// This band (or host, before a level exists) wants visual terrain pitch.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyTerrainPitch;

/// When present on the physics body (parent of the visual), pitch blends to 0.
///
/// Playgrounds copy jump-in-flight onto this marker.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuspendTerrainPitch;
