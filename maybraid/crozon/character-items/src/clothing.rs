//! Clothing item catalog and per-item color selection.

use clap::ValueEnum;

use crate::palette::ItemColor;

const CLOTHING_BASKETBALL_CUT_SHIRT: &str = "characters/clothes/basketball_cut_shirt.glb";
const CLOTHING_TUNIC: &str = "characters/clothes/tunic.glb";
const CLOTHING_LONG_DRESS: &str = "characters/clothes/long_dress.glb";
const CLOTHING_SHORT_DRESS: &str = "characters/clothes/short_dress.glb";
const CLOTHING_FITTED_COAT: &str = "characters/clothes/fitted_coat.glb";
const CLOTHING_QUARTER_COAT: &str = "characters/clothes/quarter_coat.glb";
const CLOTHING_ROBE_COAT: &str = "characters/clothes/robe_coat.glb";
const CLOTHING_SHORT_SLEEVED_ROBE_COAT: &str = "characters/clothes/short_sleeved_robe_coat.glb";
const CLOTHING_TAILORED_COAT: &str = "characters/clothes/tailored_coat.glb";
const CLOTHING_HOOD: &str = "characters/clothes/hood.glb";
const CLOTHING_PANTS: &str = "characters/clothes/pants.glb";
const CLOTHING_HAREM_PANTS: &str = "characters/clothes/harem_pants_unified.glb";
const CLOTHING_HAREM_PANTS_UPPER: &str = "characters/clothes/harem_pants_top.glb";
const CLOTHING_HAREM_PANTS_LOWER_WRAP: &str = "characters/clothes/harem_pants_bottom_wrap.glb";

/// Shared clothing catalog; layers compose across species.
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

	/// Runtime asset path relative to the `maybraid/assets` root.
	pub const fn path(self) -> &'static str {
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

/// One clothing layer's color override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothingColor {
	pub clothing: ClothingMesh,
	pub color: ItemColor,
}

impl ClothingColor {
	/// Resolves a layer's color from overrides, falling back to the default.
	pub fn resolve(
		overrides: &[ClothingColor],
		default: ItemColor,
		clothing: ClothingMesh,
	) -> ItemColor {
		overrides
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.color)
			.unwrap_or(default)
	}

	/// Sets or inserts a layer's color override.
	pub fn set(overrides: &mut Vec<ClothingColor>, clothing: ClothingMesh, color: ItemColor) {
		if let Some(choice) = overrides.iter_mut().find(|choice| choice.clothing == clothing) {
			choice.color = color;
		} else {
			overrides.push(ClothingColor { clothing, color });
		}
	}
}
