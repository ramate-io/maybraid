//! Topple asset catalog and assembly resolver.

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
			assets::{BODY_RIG, HEAD_RIG, HEAD_STANDARD},
			HairMesh,
		},
		topple::{
			pose::{TopplePose, TOPPLE_OVERALL_SCALE},
			ToppleConfig,
		},
	},
};

const BODY_WHELP: AssetPath = AssetPath::new("characters/bodies/whelp_bird.glb");
const BEAK: AssetPath = AssetPath::new("characters/snouts/beak.glb");
const HOOK_BEAK: AssetPath = AssetPath::new("characters/snouts/hook_beak.glb");
const SHARP_BEAK: AssetPath = AssetPath::new("characters/snouts/sharp_beak.glb");

/// Species-local resolver for Topple asset choices.
pub struct ToppleAssets;

impl ToppleAssets {
	pub fn resolve(config: &ToppleConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Topple",
			RigAsset::new("Humanoid", BODY_RIG)
				.with_normalization(AssetNormalization::centroid(TOPPLE_OVERALL_SCALE)),
			TopplePose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::beak(config.beak));

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
			CharacterAsset::new("whelp", BODY_WHELP, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.26)),
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
				ToppleHeadMesh::Meerkat.label(),
				HEAD_STANDARD,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn eye_right(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn beak(beak: ToppleBeakMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(beak.label(), beak.path(), AssetNormalization::centroid(0.35)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn hair(hair: HairMesh) -> Option<ResolvedCharacterPart> {
		let path = hair.path()?;
		let scale = match hair {
			HairMesh::FeatherHawk => 0.4,
			_ => 1.0,
		};
		Some(ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new(hair.label(), path, AssetNormalization::centroid(scale)),
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

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ToppleHeadMesh {
	#[default]
	Meerkat,
}

impl ToppleHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ToppleBeakMesh {
	#[default]
	Beak,
	Hook,
	Sharp,
}

impl ToppleBeakMesh {
	pub const VALUES: &'static [Self] = &[Self::Beak, Self::Hook, Self::Sharp];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Beak => "beak",
			Self::Hook => "hook",
			Self::Sharp => "sharp",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Beak => BEAK,
			Self::Hook => HOOK_BEAK,
			Self::Sharp => SHARP_BEAK,
		}
	}
}
