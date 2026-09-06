//! Per-frame character articulation: clips, mailbox, terrain pitch.
//!
//! Recipes (`crozon-characters`) stamp host identity. This crate syncs host
//! motion markers from the shown LOD band and realizes clips / pitch. See
//! [README.md](../README.md).
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
pub mod sync;

pub use clip::{
	AnimClip, AnimId, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams, TuckedFlipParams,
	TwoFootedTuckedFlipParams,
};
pub use elevation::{
	apply_terrain_pitch, draw_terrain_pitch_probes, is_local_visual_child, DrawTerrainPitchProbes,
};
pub use mailbox::{
	apply_anim_mailbox, prepare_anim_mailbox, tick_anim_mailbox, AnimBone, AnimMailbox, AnimProgress,
};
pub use markers::{
	AnimateBones, AnimateEffects, ApplyTerrainPitch, SuspendAnimation, SuspendTerrainPitch,
};
pub use pitch::TerrainPitch;
pub use plugin::{CharacterMotionPlugin, CharacterMotionSystems};
pub use policy::{motion_policy, MotionPolicy};
pub use rig::{
	bone_map_ready, missing_landmark_bones, BoneMap, CharacterRig, CharacterRigRole,
	RigSkeletonKind,
};
pub use shown::shown_level_root;
pub use sync::sync_motion_markers;
