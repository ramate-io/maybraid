//! Braidman asset catalog for the first concepts pass.
//!
//! Only a small subset is exposed so the module organization can be reviewed
//! before every Braidman feature and clothing variant is added.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::braidman::{pose::BraidmanPose, BraidmanConfig},
};

const BODY_RIG: AssetPath = AssetPath::new("characters/bodies/humanoid_rig.glb");
const BODY_STANDARD: AssetPath = AssetPath::new("characters/bodies/humanoid_full_body.glb");
const BODY_FULL: AssetPath = AssetPath::new("characters/bodies/leron_biped_full_body.glb");
const HEAD_RIG: AssetPath = AssetPath::new("characters/heads/orthograde_head.glb");
const HEAD_STANDARD: AssetPath = AssetPath::new("characters/heads/meerkat_head.glb");
const HEAD_GAUNT: AssetPath = AssetPath::new("characters/heads/gaunt_ortho_humanoid_head.glb");
const HEAD_FULL: AssetPath = AssetPath::new("characters/heads/full_ortho_humanoid_head.glb");
const EYE_STANDARD: AssetPath = AssetPath::new("characters/eyes/humanoid_eye_left.glb");
const EYE_FALCON: AssetPath = AssetPath::new("characters/eyes/falcon_eye_left.glb");
const NOSE_STANDARD: AssetPath = AssetPath::new("characters/noses/humanoid_nose.glb");
const NOSE_BROAD: AssetPath = AssetPath::new("characters/noses/broad_humanoid_nose.glb");
const NOSE_LOAF: AssetPath = AssetPath::new("characters/noses/loaf_nose.glb");
const NOSE_BALLOON: AssetPath = AssetPath::new("characters/noses/mumbus_nose.glb");
const MOUTH_STANDARD: AssetPath = AssetPath::new("characters/mouths/common_mouth.glb");
const EAR_STANDARD: AssetPath = AssetPath::new("characters/ears/round_scoop_lateral_ear_left.glb");
const EAR_ROUND: AssetPath = AssetPath::new("characters/ears/round_lateral_ear_left.glb");
const EAR_FLANK: AssetPath = AssetPath::new("characters/ears/flank_lateral_ear_left.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum BodyMesh {
	#[default]
	Standard,
	Full,
}

impl BodyMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Full => "full",
		}
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => BODY_STANDARD,
			Self::Full => BODY_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum HeadMesh {
	#[default]
	Standard,
	Gaunt,
	Full,
}

impl HeadMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Gaunt => "gaunt",
			Self::Full => "full",
		}
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => HEAD_STANDARD,
			Self::Gaunt => HEAD_GAUNT,
			Self::Full => HEAD_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum EyeMesh {
	#[default]
	Standard,
	Falcon,
}

impl EyeMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Falcon => "falcon",
		}
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EYE_STANDARD,
			Self::Falcon => EYE_FALCON,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum NoseMesh {
	#[default]
	Standard,
	Broad,
	Loaf,
	Balloon,
}

impl NoseMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Broad => "broad",
			Self::Loaf => "loaf",
			Self::Balloon => "balloon",
		}
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => NOSE_STANDARD,
			Self::Broad => NOSE_BROAD,
			Self::Loaf => NOSE_LOAF,
			Self::Balloon => NOSE_BALLOON,
		}
	}

	const fn normalization(self) -> AssetNormalization {
		match self {
			Self::Balloon => AssetNormalization::centroid(0.08),
			Self::Standard | Self::Broad | Self::Loaf => AssetNormalization::centroid(0.05),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum MouthMesh {
	#[default]
	Standard,
}

impl MouthMesh {
	pub const fn label(self) -> &'static str {
		"standard"
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => MOUTH_STANDARD,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum EarMesh {
	#[default]
	Standard,
	Round,
	Flank,
}

impl EarMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Round => "round",
			Self::Flank => "flank",
		}
	}

	const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EAR_STANDARD,
			Self::Round => EAR_ROUND,
			Self::Flank => EAR_FLANK,
		}
	}
}

/// Species-local resolver for Braidman asset choices.
pub struct BraidmanAssets;

impl BraidmanAssets {
	pub fn resolve(config: &BraidmanConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Braidman",
			RigAsset::new("Humanoid", BODY_RIG),
			BraidmanPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh(config.body))
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose(config.nose))
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::ear_left(config.ear))
		.with_part(Self::ear_right(config.ear))
	}

	fn body_mesh(body: BodyMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(body.label(), body.path(), AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.12)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh(head: HeadMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(head.label(), head.path(), AssetNormalization::IDENTITY),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.04)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("eye.L", Transform::IDENTITY)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.04)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("eye.R", Self::mirror_x())),
		)
	}

	fn nose(nose: NoseMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(nose.label(), nose.path(), nose.normalization()),
			SkinTarget::HeadRig,
			Some(Self::head_socket("nose", Transform::IDENTITY)),
		)
	}

	fn mouth(mouth: MouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.02)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("mouth", Transform::IDENTITY)),
		)
	}

	fn ear_left(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new(
				ear.label(),
				ear.path(),
				AssetNormalization::centroid(0.08).facing_positive_x(),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("cheek.L", Transform::IDENTITY)),
		)
	}

	fn ear_right(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new(
				ear.label(),
				ear.path(),
				AssetNormalization::centroid(0.08).facing_positive_x(),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("cheek.R", Self::mirror_x())),
		)
	}

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}
