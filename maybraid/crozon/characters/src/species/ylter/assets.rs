//! Yilter asset catalog: Hars neck stack + Dui barred-bowl head + cow snout.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::{
			BODY_RUMBLER, HEAD_RIG, MOUTH_COW_SNOUT, NECK_BASIC, NECK_TRIPLE_JOIN, QUADRUPED_RIG,
			TAIL_CAT,
		},
		ylter::{pose::YilterPose, YilterConfig},
	},
};

const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
pub(crate) const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");

/// Species-local resolver for Yilter asset choices.
pub struct YilterAssets;

impl YilterAssets {
	pub fn resolve(config: &YilterConfig) -> ResolvedCharacterAssembly {
		let pose = YilterPose::from_config(config);
		ResolvedCharacterAssembly::new(
			"Yilter",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			pose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::neck_rig(pose))
		.with_part(Self::neck_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left())
		.with_part(Self::eye_right())
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::tail())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				YilterBodyMesh::Rumbler.label(),
				BODY_RUMBLER,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn neck_rig(pose: YilterPose) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::NeckRig,
			CharacterAsset::new(
				"TripleJoinNeck",
				NECK_TRIPLE_JOIN,
				AssetNormalization::base_y(0.4),
			),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "head_socket",
				local_transform: Transform::from_translation(Vec3::new(0.0, 0.2, -0.2)),
			}),
		)
		.with_pose(pose.neck_pose())
	}

	fn neck_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::NeckMesh,
			CharacterAsset::new("basic-neck", NECK_BASIC, AssetNormalization::IDENTITY),
			SkinTarget::NeckRig,
			Some(SocketAttachment {
				rig: SocketRig::Neck,
				bone: "neck_base",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(1.2)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Neck,
				bone: "head_socket",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				YilterHeadMesh::BarredBowl.label(),
				HEAD_BARRED_BOWL,
				AssetNormalization::base_y(1.2),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new("thorn", EYE_THORN, AssetNormalization::centroid(0.6)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.05)),
			)),
		)
	}

	fn eye_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new("thorn", EYE_THORN, AssetNormalization::centroid(0.6)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.1, 0.05)),
			)),
		)
	}

	fn mouth(mouth: YilterMouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.3)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_4))
					.with_translation(Vec3::new(0.0, -0.15, -0.1))
					.with_scale(Vec3::new(4.0, 2.0, 2.0)),
			)),
		)
	}

	fn tail() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Tail,
			CharacterAsset::new("cat-tail", TAIL_CAT, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			Some(Self::body_socket("tailbone", Transform::IDENTITY)),
		)
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
pub enum YilterBodyMesh {
	#[default]
	Rumbler,
}

impl YilterBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Rumbler];

	pub const fn label(self) -> &'static str {
		"rumbler"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_RUMBLER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum YilterHeadMesh {
	#[default]
	BarredBowl,
}

impl YilterHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum YilterMouthMesh {
	#[default]
	Cow,
}

impl YilterMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
