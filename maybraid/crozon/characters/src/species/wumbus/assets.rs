//! Wumbus asset catalog and assembly resolver.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::assets::{
			BODY_RIG, EAR_FLANK, HEAD_ORTHO_BEAR, HEAD_RIG, HORNS_HARROWED_CROWN, MOUTH_CANINE_SNOUT,
		},
		wumbus::{pose::WumbusPose, WumbusConfig},
	},
};

const BODY_WUMBUS: AssetPath = AssetPath::new("characters/bodies/wumbus_biped_full_body.glb");

/// Species-local resolver for Wumbus asset choices.
pub struct WumbusAssets;

impl WumbusAssets {
	pub fn resolve(config: &WumbusConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Wumbus",
			RigAsset::new("Humanoid", BODY_RIG),
			WumbusPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::mouth())
		.with_part(Self::ear_left())
		.with_part(Self::ear_right());

		let assembly = if config.horns != WumbusHornMesh::None {
			assembly.with_part(Self::horns(config.horns))
		} else {
			assembly
		};
		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config
			.clothing
			.iter()
			.fold(assembly, |assembly, clothing| assembly.with_part(Self::clothing(*clothing)))
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("wumbus", BODY_WUMBUS, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.3)),
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
				WumbusHeadMesh::OrthoBear.label(),
				HEAD_ORTHO_BEAR,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.12)),
			)),
		)
	}

	fn eye_right(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, 0.0, -0.12)),
			)),
		)
	}

	fn mouth() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(
				WumbusMouthMesh::CanineSnout.label(),
				MOUTH_CANINE_SNOUT,
				AssetNormalization::centroid(0.4),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn ear_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				Transform::from_translation(Vec3::new(0.15, 0.3, -0.05))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn ear_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				Self::mirror_x()
					.with_translation(Vec3::new(-0.15, 0.3, -0.05))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn horns(horns: WumbusHornMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(horns.label(), horns.path(), AssetNormalization::centroid(0.7)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
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

	fn clothing(clothing: crate::species::common::ClothingMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Clothing,
			CharacterAsset::new(clothing.label(), clothing.path(), AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusHeadMesh {
	#[default]
	OrthoBear,
}

impl WumbusHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoBear];

	pub const fn label(self) -> &'static str {
		"ortho-bear"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_ORTHO_BEAR
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusMouthMesh {
	#[default]
	CanineSnout,
}

impl WumbusMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::CanineSnout];

	pub const fn label(self) -> &'static str {
		"canine-snout"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_CANINE_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl WumbusHornMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::HarrowedCrown => "harrowed-crown",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::None => HORNS_HARROWED_CROWN,
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
		}
	}
}
