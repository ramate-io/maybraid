//! Braidman asset catalog for the concepts playground.
//!
//! Phase 2 adds hair and clothing through the same resolved-part path as body
//! and head features: hair socketed on the head rig `crown` bone, clothing
//! remapped to the body rig. Clothing is multi-select via `BraidmanConfig::clothing`.

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
const HAIR_THICK_BRAIDS: AssetPath = AssetPath::new("characters/hair/thick_braids.glb");
const HAIR_FLOWING_CURLS: AssetPath = AssetPath::new("characters/hair/flowing_curls.glb");
const HAIR_WRAPPING_BRAIDS: AssetPath = AssetPath::new("characters/hair/wrapping_braids.glb");
const HAIR_WRAPPING_BRAIDS_HANGING_LOCKS: AssetPath =
	AssetPath::new("characters/hair/wrapping_braids_hanging_locks.glb");
const HAIR_BRAID_HAWK: AssetPath = AssetPath::new("characters/hair/braid_hawk.glb");
const HAIR_FEATHER_HAWK: AssetPath = AssetPath::new("characters/hair/feather_hawk.glb");
const HAIR_FLOWING_EDGY_CURLS: AssetPath = AssetPath::new("characters/hair/flowing_edgy_curls.glb");
const HAIR_PERM_BRAID: AssetPath = AssetPath::new("characters/hair/perm_braid.glb");
const HAIR_TECHNO_EDGE: AssetPath = AssetPath::new("characters/hair/techno_edge.glb");
const CLOTHING_BASKETBALL_CUT_SHIRT: AssetPath =
	AssetPath::new("characters/clothes/basketball_cut_shirt.glb");
const CLOTHING_TUNIC: AssetPath = AssetPath::new("characters/clothes/tunic.glb");
const CLOTHING_LONG_DRESS: AssetPath = AssetPath::new("characters/clothes/long_dress.glb");
const CLOTHING_SHORT_DRESS: AssetPath = AssetPath::new("characters/clothes/short_dress.glb");
const CLOTHING_FITTED_COAT: AssetPath = AssetPath::new("characters/clothes/fitted_coat.glb");
const CLOTHING_QUARTER_COAT: AssetPath = AssetPath::new("characters/clothes/quarter_coat.glb");
const CLOTHING_ROBE_COAT: AssetPath = AssetPath::new("characters/clothes/robe_coat.glb");
const CLOTHING_SHORT_SLEEVED_ROBE_COAT: AssetPath =
	AssetPath::new("characters/clothes/short_sleeved_robe_coat.glb");
const CLOTHING_TAILORED_COAT: AssetPath = AssetPath::new("characters/clothes/tailored_coat.glb");
const CLOTHING_HOOD: AssetPath = AssetPath::new("characters/clothes/hood.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
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

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => BODY_STANDARD,
			Self::Full => BODY_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
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

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => HEAD_STANDARD,
			Self::Gaunt => HEAD_GAUNT,
			Self::Full => HEAD_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
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

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EYE_STANDARD,
			Self::Falcon => EYE_FALCON,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
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

	pub const fn path(self) -> AssetPath {
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
			Self::Standard | Self::Broad | Self::Loaf => AssetNormalization::centroid(0.2),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum MouthMesh {
	#[default]
	Standard,
}

impl MouthMesh {
	pub const fn label(self) -> &'static str {
		"standard"
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => MOUTH_STANDARD,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
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

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EAR_STANDARD,
			Self::Round => EAR_ROUND,
			Self::Flank => EAR_FLANK,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HairMesh {
	#[default]
	None,
	ThickBraids,
	FlowingCurls,
	WrappingBraids,
	WrappingBraidsHangingLocks,
	BraidHawk,
	FeatherHawk,
	FlowingEdgyCurls,
	PermBraid,
	TechnoEdge,
}

impl HairMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::ThickBraids => "thick-braids",
			Self::FlowingCurls => "flowing-curls",
			Self::WrappingBraids => "wrapping-braids",
			Self::WrappingBraidsHangingLocks => "wrapping-braids-hanging-locks",
			Self::BraidHawk => "braid-hawk",
			Self::FeatherHawk => "feather-hawk",
			Self::FlowingEdgyCurls => "flowing-edgy-curls",
			Self::PermBraid => "perm-braid",
			Self::TechnoEdge => "techno-edge",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::ThickBraids => Some(HAIR_THICK_BRAIDS),
			Self::FlowingCurls => Some(HAIR_FLOWING_CURLS),
			Self::WrappingBraids => Some(HAIR_WRAPPING_BRAIDS),
			Self::WrappingBraidsHangingLocks => Some(HAIR_WRAPPING_BRAIDS_HANGING_LOCKS),
			Self::BraidHawk => Some(HAIR_BRAID_HAWK),
			Self::FeatherHawk => Some(HAIR_FEATHER_HAWK),
			Self::FlowingEdgyCurls => Some(HAIR_FLOWING_EDGY_CURLS),
			Self::PermBraid => Some(HAIR_PERM_BRAID),
			Self::TechnoEdge => Some(HAIR_TECHNO_EDGE),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClothingMesh {
	BasketballCutShirt,
	Tunic,
	LongDress,
	ShortDress,
	FittedCoat,
	QuarterCoat,
	RobeCoat,
	ShortSleevedRobeCoat,
	TailoredCoat,
	Hood,
}

impl ClothingMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::BasketballCutShirt => "basketball-cut-shirt",
			Self::Tunic => "tunic",
			Self::LongDress => "long-dress",
			Self::ShortDress => "short-dress",
			Self::FittedCoat => "fitted-coat",
			Self::QuarterCoat => "quarter-coat",
			Self::RobeCoat => "robe-coat",
			Self::ShortSleevedRobeCoat => "short-sleeved-robe-coat",
			Self::TailoredCoat => "tailored-coat",
			Self::Hood => "hood",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::BasketballCutShirt => CLOTHING_BASKETBALL_CUT_SHIRT,
			Self::Tunic => CLOTHING_TUNIC,
			Self::LongDress => CLOTHING_LONG_DRESS,
			Self::ShortDress => CLOTHING_SHORT_DRESS,
			Self::FittedCoat => CLOTHING_FITTED_COAT,
			Self::QuarterCoat => CLOTHING_QUARTER_COAT,
			Self::RobeCoat => CLOTHING_ROBE_COAT,
			Self::ShortSleevedRobeCoat => CLOTHING_SHORT_SLEEVED_ROBE_COAT,
			Self::TailoredCoat => CLOTHING_TAILORED_COAT,
			Self::Hood => CLOTHING_HOOD,
		}
	}
}

/// Species-local resolver for Braidman asset choices.
pub struct BraidmanAssets;

impl BraidmanAssets {
	pub fn resolve(config: &BraidmanConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Braidman",
			RigAsset::new("Humanoid", BODY_RIG),
			BraidmanPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh(config.body))
		// Head rig is an armature scene, not a head mesh variant selector.
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose(config.nose))
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::ear_left(config.ear))
		.with_part(Self::ear_right(config.ear));

		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config
			.clothing
			.iter()
			.fold(assembly, |assembly, clothing| assembly.with_part(Self::clothing(*clothing)))
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
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.26)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh(head: HeadMesh) -> ResolvedCharacterPart {
		// Mesh variant skins to the head rig; it does not replace the rig asset.
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
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
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

	fn nose(nose: NoseMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(nose.label(), nose.path(), nose.normalization()),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn mouth(mouth: MouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.12)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn ear_left(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.15)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				// Bump out and angle back 45 degrees.
				Transform::from_translation(Vec3::new(0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn ear_right(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.15)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				// Bump out and angle back 45 degrees.
				Self::mirror_x()
					.with_translation(Vec3::new(-0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn hair(hair: HairMesh) -> Option<ResolvedCharacterPart> {
		let path = hair.path()?;
		Some(ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new(hair.label(), path, AssetNormalization::centroid(1.0)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				// shift down a bit to compensate for the head rig's base-y anchor
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		))
	}

	fn clothing(clothing: ClothingMesh) -> ResolvedCharacterPart {
		// Each layer remaps independently onto the body rig; spec fit is NoChanges.
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
		// Left-authored GLBs: mirror X for the right-side instance without duplicating assets.
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}
