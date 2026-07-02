//! Shared humanoid asset paths and mesh enums used by multiple species.

use clap::ValueEnum;

use crate::assets::AssetPath;

pub const BODY_RIG: AssetPath = AssetPath::new("characters/bodies/humanoid_rig.glb");
pub const BODY_STANDARD: AssetPath = AssetPath::new("characters/bodies/humanoid_full_body.glb");
pub const BODY_FULL: AssetPath = AssetPath::new("characters/bodies/leron_biped_full_body.glb");
pub const HEAD_RIG: AssetPath = AssetPath::new("characters/heads/orthograde_head.glb");
pub const HEAD_STANDARD: AssetPath = AssetPath::new("characters/heads/meerkat_head_v2.glb");
pub const HEAD_GAUNT: AssetPath = AssetPath::new("characters/heads/gaunt_ortho_humanoid_head.glb");
pub const HEAD_FULL: AssetPath = AssetPath::new("characters/heads/full_ortho_humanoid_head.glb");
pub const EYE_STANDARD: AssetPath = AssetPath::new("characters/eyes/humanoid_eye_left.glb");
pub const EYE_FALCON: AssetPath = AssetPath::new("characters/eyes/falcon_eye_left.glb");
pub const NOSE_STANDARD: AssetPath = AssetPath::new("characters/noses/humanoid_nose.glb");
pub const NOSE_BROAD: AssetPath = AssetPath::new("characters/noses/broad_humanoid_nose.glb");
pub const NOSE_LOAF: AssetPath = AssetPath::new("characters/noses/loaf_nose.glb");
pub const NOSE_BALLOON: AssetPath = AssetPath::new("characters/noses/mumbus_nose.glb");
pub const MOUTH_STANDARD: AssetPath = AssetPath::new("characters/mouths/common_mouth.glb");
pub const EAR_STANDARD: AssetPath =
	AssetPath::new("characters/ears/round_scoop_lateral_ear_left.glb");
pub const EAR_ROUND: AssetPath = AssetPath::new("characters/ears/round_lateral_ear_left.glb");
pub const EAR_FLANK: AssetPath = AssetPath::new("characters/ears/flank_lateral_ear_left.glb");
pub const HORNS_HARROWED_CROWN: AssetPath = AssetPath::new("characters/horns/harrowed_crown.glb");
pub const HORNS_LORKEN_CROWN: AssetPath = AssetPath::new("characters/horns/lorken_crown.glb");
pub const HAIR_THICK_BRAIDS: AssetPath = AssetPath::new("characters/hair/thick_braids.glb");
pub const HAIR_FLOWING_CURLS: AssetPath = AssetPath::new("characters/hair/flowing_curls.glb");
pub const HAIR_WRAPPING_BRAIDS: AssetPath = AssetPath::new("characters/hair/wrapping_braids.glb");
pub const HAIR_WRAPPING_BRAIDS_HANGING_LOCKS: AssetPath =
	AssetPath::new("characters/hair/wrapping_braids_hanging_locks.glb");
pub const HAIR_BRAID_HAWK: AssetPath = AssetPath::new("characters/hair/braid_hawk.glb");
pub const HAIR_FEATHER_HAWK: AssetPath = AssetPath::new("characters/hair/feather_hawk.glb");
pub const HAIR_FLOWING_EDGY_CURLS: AssetPath =
	AssetPath::new("characters/hair/flowing_edgy_curls.glb");
pub const HAIR_PERM_BRAID: AssetPath = AssetPath::new("characters/hair/perm_braid.glb");
pub const HAIR_TECHNO_EDGE: AssetPath = AssetPath::new("characters/hair/techno_edge.glb");
pub const CLOTHING_BASKETBALL_CUT_SHIRT: AssetPath =
	AssetPath::new("characters/clothes/basketball_cut_shirt.glb");
pub const CLOTHING_TUNIC: AssetPath = AssetPath::new("characters/clothes/tunic.glb");
pub const CLOTHING_LONG_DRESS: AssetPath = AssetPath::new("characters/clothes/long_dress.glb");
pub const CLOTHING_SHORT_DRESS: AssetPath = AssetPath::new("characters/clothes/short_dress.glb");
pub const CLOTHING_FITTED_COAT: AssetPath = AssetPath::new("characters/clothes/fitted_coat.glb");
pub const CLOTHING_QUARTER_COAT: AssetPath = AssetPath::new("characters/clothes/quarter_coat.glb");
pub const CLOTHING_ROBE_COAT: AssetPath = AssetPath::new("characters/clothes/robe_coat.glb");
pub const CLOTHING_SHORT_SLEEVED_ROBE_COAT: AssetPath =
	AssetPath::new("characters/clothes/short_sleeved_robe_coat.glb");
pub const CLOTHING_TAILORED_COAT: AssetPath =
	AssetPath::new("characters/clothes/tailored_coat.glb");
pub const CLOTHING_HOOD: AssetPath = AssetPath::new("characters/clothes/hood.glb");
pub const CLOTHING_PANTS: AssetPath = AssetPath::new("characters/clothes/pants.glb");
pub const CLOTHING_HAREM_PANTS: AssetPath =
	AssetPath::new("characters/clothes/harem_pants_unified.glb");
pub const CLOTHING_HAREM_PANTS_UPPER: AssetPath =
	AssetPath::new("characters/clothes/harem_pants_top.glb");
pub const CLOTHING_HAREM_PANTS_LOWER_WRAP: AssetPath =
	AssetPath::new("characters/clothes/harem_pants_bottom_wrap.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BodyMesh {
	#[default]
	Standard,
	Full,
}

impl BodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Full];

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
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Gaunt, Self::Full];

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
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Falcon];

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
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Broad, Self::Loaf, Self::Balloon];

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

	pub const fn normalization(self) -> crate::assets::AssetNormalization {
		use crate::assets::AssetNormalization;
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
	pub const VALUES: &'static [Self] = &[Self::Standard];

	pub const fn label(self) -> &'static str {
		"standard"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_STANDARD
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
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Round, Self::Flank];

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
	pub const VALUES: &'static [Self] = &[
		Self::None,
		Self::ThickBraids,
		Self::FlowingCurls,
		Self::WrappingBraids,
		Self::WrappingBraidsHangingLocks,
		Self::BraidHawk,
		Self::FeatherHawk,
		Self::FlowingEdgyCurls,
		Self::PermBraid,
		Self::TechnoEdge,
	];

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
	Pants,
	HaremPants,
	HaremPantsUpper,
	HaremPantsLowerWrap,
}

impl ClothingMesh {
	pub const VALUES: &'static [Self] = &[
		Self::BasketballCutShirt,
		Self::Tunic,
		Self::LongDress,
		Self::ShortDress,
		Self::FittedCoat,
		Self::QuarterCoat,
		Self::RobeCoat,
		Self::ShortSleevedRobeCoat,
		Self::TailoredCoat,
		Self::Hood,
		Self::Pants,
		Self::HaremPants,
		Self::HaremPantsUpper,
		Self::HaremPantsLowerWrap,
	];

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
			Self::Pants => "pants",
			Self::HaremPants => "harem-pants",
			Self::HaremPantsUpper => "harem-pants-upper",
			Self::HaremPantsLowerWrap => "harem-pants-lower-wrap",
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
			Self::Pants => CLOTHING_PANTS,
			Self::HaremPants => CLOTHING_HAREM_PANTS,
			Self::HaremPantsUpper => CLOTHING_HAREM_PANTS_UPPER,
			Self::HaremPantsLowerWrap => CLOTHING_HAREM_PANTS_LOWER_WRAP,
		}
	}
}
