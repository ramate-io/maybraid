//! Resolved character assembly data.
//!
//! A species resolver produces these values after commands or future UI fields
//! have been normalized. The data describes what should be spawned and how parts
//! relate to rigs; it deliberately does not issue Bevy commands itself.

use bevy::prelude::*;

use crate::assets::{AssetNormalization, AssetPath};
use crozon_rigs::ResolvedRigPose;

/// A rig scene used as an animation or skinning target.
#[derive(Debug, Clone, PartialEq)]
pub struct RigAsset {
	pub label: &'static str,
	pub path: AssetPath,
}

impl RigAsset {
	pub const fn new(label: &'static str, path: AssetPath) -> Self {
		Self { label, path }
	}
}

/// A mesh or feature asset after species/preset/command resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterAsset {
	pub label: &'static str,
	pub path: AssetPath,
	pub normalization: AssetNormalization,
}

impl CharacterAsset {
	pub const fn new(
		label: &'static str,
		path: AssetPath,
		normalization: AssetNormalization,
	) -> Self {
		Self { label, path, normalization }
	}
}

/// Semantic slot used for debugging and future UI/status reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CharacterPartSlot {
	#[default]
	BodyMesh,
	HeadRig,
	HeadMesh,
	EyeLeft,
	EyeRight,
	Nose,
	Mouth,
	EarLeft,
	EarRight,
	Hair,
	Horns,
	Clothing,
	Spine,
	Tail,
}

/// Which rig should receive a skinned part's joint remap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SkinTarget {
	#[default]
	BodyRig,
	HeadRig,
	/// Part keeps its embedded armature (e.g. head rig scene before socket attach).
	OwnRig,
	/// Socketed prop with no skinning, or mesh follows parent transform only.
	None,
}

/// The rig hierarchy that owns a socket bone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SocketRig {
	#[default]
	Body,
	Head,
}

/// Placement of a resolved part onto a named bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SocketAttachment {
	pub rig: SocketRig,
	pub bone: &'static str,
	pub local_transform: Transform,
}

impl SocketAttachment {
	pub const fn new(rig: SocketRig, bone: &'static str, local_transform: Transform) -> Self {
		Self { rig, bone, local_transform }
	}
}

/// A resolved part ready for a preview spawner.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCharacterPart {
	pub slot: CharacterPartSlot,
	pub asset: CharacterAsset,
	pub skin_target: SkinTarget,
	pub socket: Option<SocketAttachment>,
}

impl ResolvedCharacterPart {
	pub const fn new(
		slot: CharacterPartSlot,
		asset: CharacterAsset,
		skin_target: SkinTarget,
		socket: Option<SocketAttachment>,
	) -> Self {
		Self { slot, asset, skin_target, socket }
	}

	/// Adapter from the shared item catalog: clothing skins onto the body rig.
	pub const fn clothing(clothing: crozon_character_items::ClothingMesh) -> Self {
		Self::new(
			CharacterPartSlot::Clothing,
			CharacterAsset::new(
				clothing.label(),
				AssetPath::new(clothing.path()),
				AssetNormalization::IDENTITY,
			),
			SkinTarget::BodyRig,
			None,
		)
	}
}

/// Complete resolved preview assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCharacterAssembly {
	pub label: &'static str,
	pub body_rig: RigAsset,
	pub parts: Vec<ResolvedCharacterPart>,
	/// Proportional layers for the body rig; head rig pose is future work.
	pub pose: ResolvedRigPose,
}

impl ResolvedCharacterAssembly {
	pub fn new(label: &'static str, body_rig: RigAsset, pose: ResolvedRigPose) -> Self {
		Self { label, body_rig, parts: Vec::new(), pose }
	}

	pub fn with_part(mut self, part: ResolvedCharacterPart) -> Self {
		self.parts.push(part);
		self
	}

	pub fn parts(&self) -> impl Iterator<Item = &ResolvedCharacterPart> {
		self.parts.iter()
	}

	pub fn part_count(&self) -> usize {
		self.parts.len()
	}
}
