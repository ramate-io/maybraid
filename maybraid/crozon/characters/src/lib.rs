//! Reusable Crozon character definitions.
//!
//! [`CharacterComponents`] is the character recipe: species configs produce nested
//! [`lod::LodScene`] hosts ([`ComponentsOnly`], [`RigNode`], [`PartNode`]), with
//! sockets and skinning as deferred refs parallel to [`scene_ref::SceneRef`].
//! Live pose, animation, and paint are ECS mutation (`MemberOf` + `*Ref` +
//! `Changed`), not LOD refresh. Per-frame clips and terrain pitch live in
//! [`crozon_character_motion`]; this crate stamps **initial** host markers from
//! [`lod::LodScene::host`] / spawn. Motion sync keeps those markers aligned with
//! the shown LOD band.

pub mod anim;
pub mod appearance;
pub mod assembly;
pub mod assets;
pub mod components;
pub mod concepts;
pub mod hosts;
pub mod layer;
pub mod material_lib;
pub mod member;
pub mod menu_traits;
pub mod nodes;
pub mod plugin;
pub mod pose;
pub mod presets;
pub mod rig;
pub mod scene_children;
pub mod skin;
pub mod socket;
pub mod species;
pub mod terrain_pitch;

pub use anim::{
	apply_anim_mailbox, prepare_anim_mailbox, tick_anim_mailbox, AnimBone, AnimClip, AnimId,
	AnimMailbox, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams, TuckedFlipParams,
	TwoFootedTuckedFlipParams,
};
pub use appearance::CharacterAppearance;
pub use assembly::CharacterPartSlot;
pub use assets::{AssetFacing, AssetNormalization, AssetPath, AuthoredAnchor};
pub use components::{
	character_bounds, clothing_layers, CharacterComponents, CharacterRecipe, Clothed,
	ClothingLayer, ComponentsOnly,
};
pub use concepts::ConceptAnimation;
pub use crozon_character_motion::{
	apply_terrain_pitch, motion_policy, sync_motion_markers, AnimateBones, AnimateEffects,
	ApplyTerrainPitch, CharacterMotionPlugin, CharacterMotionSystems, MotionPolicy,
	SuspendTerrainPitch,
};
pub use crozon_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};
pub use hosts::CharacterHostsPlugin;
pub use layer::{Layer, Layers};
pub use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
pub use member::{
	attach_part_node, find_member_rig, find_part_member, hide_socketed_parts,
	stamp_character_members, CharacterMembers, CharacterRoot, MemberOf,
};
pub use nodes::{PartNode, RigNode};
pub use plugin::{add_character_components_host, CharacterComponentsPlugin, CharacterHostSystems};
pub use pose::maintain_resolved_pose;
pub use presets::{BuildPreset, GenderPreset};
pub use rig::{
	bind_scales_ready, bone_map_ready, build_rig_bone_map, missing_landmark_bones, ActiveRigPose,
	BoneMap, CharacterPart, CharacterRig, CharacterRigRole, LodCharacterRig,
	NeedsDuplicateScenePrune, NeedsSkinRemap, NoMatchingArmature, PartRigRef, ResolvedPoseApplied,
	RigBindScales, RigSkeletonKind,
};
pub use rigs::{AssemblyHost, AssemblyRoot, RigKey, RigPlugin, RigRoot, RigSystems};
pub use skin::{
	fulfill_skin_ref_roots, invalidate_changed_skin_ref_roots, prune_duplicate_part_scenes,
	remap_part_skin_to_rig,
};
pub use socket::{
	fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots, RigId, SkinRef, SkinRefApplied,
	SkinRefRoot, SocketRef, SocketRefApplied, SocketRefRoot,
};
pub use terrain_pitch::prepare_character_terrain_pitch;
pub use terrain_pitch::TerrainPitch;
