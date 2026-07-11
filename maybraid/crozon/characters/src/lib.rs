//! Reusable Crozon character definitions.
//!
//! This crate owns the data model that sits between a character creation input
//! surface and any concrete Bevy preview. Commands and future UI should resolve
//! into these types first, then a playground or game runtime can decide how to
//! spawn, skin, animate, and inspect the resulting assembly.

pub mod assembly;
pub mod assets;
pub mod concepts;
pub mod menu_traits;
pub mod presets;
pub mod species;

pub use assembly::{
	CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart, RigAsset,
	SkinTarget, SocketAttachment, SocketRig,
};
pub use assets::{AssetFacing, AssetNormalization, AssetPath, AuthoredAnchor};
pub use concepts::ConceptAnimation;
pub use crozon_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};
pub use presets::{BuildPreset, GenderPreset};
