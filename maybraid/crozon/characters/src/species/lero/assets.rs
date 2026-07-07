//! Lero asset catalog and assembly resolver.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::assets::{BODY_FULL, BODY_RIG, EYE_STANDARD, HEAD_RIG},
		lero::{pose::LeroPose, LeroConfig},
	},
};

const HEAD_ORTHO_TEE: AssetPath = AssetPath::new("characters/heads/ortho_tee_head.glb");
const SNOUT_LERODON: AssetPath = AssetPath::new("characters/snouts/lerodon_snout.glb");
const SNOUT_ROBREK: AssetPath = AssetPath::new("characters/snouts/robrek_snout.glb");
const TAIL_LERODON: AssetPath = AssetPath::new("characters/tails/lerodon_tail.glb");
const SPINE_LERODON: AssetPath = AssetPath::new("characters/spines/spiked_lerodon_full_exo.glb");

const SNOUT_Z_SCALE: f32 = 2.5;

/// Species-local resolver for Lero asset choices.
pub struct LeroAssets;

impl LeroAssets {
	pub fn resolve(config: &LeroConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Lero",
			RigAsset::new("Humanoid", BODY_RIG),
			LeroPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left())
		.with_part(Self::eye_right())
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::tail())
		.with_part(Self::spine());

		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config.clothing.iter().fold(assembly, |assembly, clothing| {
			assembly.with_part(ResolvedCharacterPart::clothing(*clothing))
		})
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("leron", BODY_FULL, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.28)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				LeroHeadMesh::OrthoTee.label(),
				HEAD_ORTHO_TEE,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new("standard", EYE_STANDARD, AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.05, -0.12)),
			)),
		)
	}

	fn eye_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new("standard", EYE_STANDARD, AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.05, -0.12)),
			)),
		)
	}

	fn mouth(mouth: LeroMouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)).with_scale(Vec3::new(
					1.0,
					1.0,
					SNOUT_Z_SCALE,
				)),
			)),
		)
	}

	fn tail() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Tail,
			CharacterAsset::new("lerodon-tail", TAIL_LERODON, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			Some(Self::body_socket("root", Transform::IDENTITY)),
		)
	}

	fn spine() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Spine,
			CharacterAsset::new("lerodon-spine", SPINE_LERODON, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			Some(Self::body_socket("upper_back", Transform::IDENTITY)),
		)
	}

	fn hair(hair: crate::species::common::HairMesh) -> Option<ResolvedCharacterPart> {
		let path = hair.path()?;
		Some(ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new(hair.label(), path, AssetNormalization::centroid(1.0)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		))
	}

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn body_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Body, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroHeadMesh {
	#[default]
	OrthoTee,
}

impl LeroHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoTee];

	pub const fn label(self) -> &'static str {
		"ortho-tee"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_ORTHO_TEE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroMouthMesh {
	#[default]
	Lerodon,
	Robrek,
}

impl LeroMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Lerodon, Self::Robrek];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Lerodon => "lerodon",
			Self::Robrek => "robrek",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Lerodon => SNOUT_LERODON,
			Self::Robrek => SNOUT_ROBREK,
		}
	}
}
