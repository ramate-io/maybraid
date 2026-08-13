//! Reusable Crozon character definitions.
//!
//! This crate owns the data model that sits between a character creation input
//! surface and any concrete Bevy preview. Commands and future UI should resolve
//! into these types first, then a playground or game runtime can decide how to
//! spawn, skin, animate, and inspect the resulting assembly.
//!
//! Species that implement [`CharacterComponents`] present as nested [`lod::LodScene`]
//! hosts ([`ComponentsOnly`], [`RigNode`], [`PartNode`]), with sockets and skinning
//! as deferred refs parallel to [`scene_ref::SceneRef`].

pub mod assembly;
pub mod assets;
pub mod components;
pub mod concepts;
pub mod layer;
pub mod menu_traits;
pub mod nodes;
pub mod plugin;
pub mod presets;
pub mod rig;
pub mod scene_children;
pub mod skin;
pub mod socket;
pub mod species;

pub use assembly::{
	CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart, RigAsset,
	SkinTarget, SocketAttachment, SocketRig,
};
pub use assets::{AssetFacing, AssetNormalization, AssetPath, AuthoredAnchor};
pub use components::{
	character_bounds, clothing_layers, spawn_character_components, CharacterComponents,
	CharacterRecipe, Clothed, ClothingLayer, ComponentsOnly,
};
pub use concepts::ConceptAnimation;
pub use crozon_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};
pub use layer::{Layer, Layers};
pub use nodes::{PartNode, RigNode};
pub use plugin::{add_character_components_host, CharacterComponentsPlugin, CharacterHostSystems};
pub use presets::{BuildPreset, GenderPreset};
pub use rig::{
	bind_scales_ready, bone_map_ready, build_rig_bone_map, missing_landmark_bones, ActiveRigPose,
	BoneMap, CharacterPart, CharacterRig, CharacterRigRole, LodCharacterRig,
	NeedsDuplicateScenePrune, NeedsSkinRemap, NoMatchingArmature, PartRigRef, ResolvedPoseApplied,
	RigBindScales, RigSkeletonKind,
};
pub use skin::{fulfill_skin_ref_roots, prune_duplicate_part_scenes, remap_part_skin_to_rig};
pub use socket::{
	fulfill_socket_ref_roots, RigId, SkinRef, SkinRefApplied, SkinRefRoot, SocketRef,
	SocketRefApplied, SocketRefRoot,
};
