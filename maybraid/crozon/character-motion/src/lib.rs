//! Per-frame character articulation: clips, mailbox, terrain pitch.
//!
//! Recipes (`crozon-characters`) stamp identity. This crate realizes it. See
//! [README.md](../README.md) for the dataflow and how to add behaviors.
//!
//! This crate does **not** implement [`lod::LodScene`] or species recipes.

pub mod clip;
pub mod elevation;
pub mod mailbox;
pub mod markers;
pub mod pitch;
pub mod plugin;
pub mod policy;
pub mod rig;
pub mod shown;

pub use clip::{
	AnimClip, AnimId, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams, TuckedFlipParams,
	TwoFootedTuckedFlipParams,
};
pub use elevation::apply_terrain_pitch;
pub use mailbox::{prepare_anim_mailbox, tick_anim_mailbox, AnimBone, AnimMailbox};
pub use markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch, SuspendTerrainPitch};
pub use pitch::TerrainPitch;
pub use plugin::{CharacterMotionPlugin, CharacterMotionSystems};
pub use policy::{motion_policy, MotionPolicy};
pub use rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};
pub use shown::{shown_level_has, shown_level_root};
