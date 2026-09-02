//! Clothing item catalog, per-host fit paths, and per-item color selection.

use clap::ValueEnum;

use crate::palette::ItemColor;

const CLOTHING_TANK_TOP: &str = "characters/clothes/body/tank_top.glb";
const CLOTHING_TUNIC: &str = "characters/clothes/body/tunic.glb";
const CLOTHING_LONG_DRESS: &str = "characters/clothes/body/long_dress.glb";
const CLOTHING_SHORT_DRESS: &str = "characters/clothes/body/short_dress.glb";
const CLOTHING_FITTED_COAT: &str = "characters/clothes/body/fitted_coat.glb";
const CLOTHING_ROBE_COAT: &str = "characters/clothes/body/robe_coat.glb";
const CLOTHING_ROBE: &str = "characters/clothes/body/robe.glb";
const CLOTHING_HOOD: &str = "characters/clothes/head/hood.glb";
const CLOTHING_PANTS: &str = "characters/clothes/body/pants.glb";
const CLOTHING_KNEE_HIGH_BOOTS: &str = "characters/clothes/body/knee_high_boots.glb";
const CLOTHING_HAREM_PANTS: &str = "characters/clothes/body/harem_pants_unified.glb";
const CLOTHING_HAREM_PANTS_UPPER: &str = "characters/clothes/body/harem_pants_top.glb";
const CLOTHING_HAREM_PANTS_LOWER_WRAP: &str = "characters/clothes/body/harem_pants_bottom_wrap.glb";

/// Which character part a garment is wrapped onto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClothingSlot {
	Body,
	Head,
}

impl ClothingSlot {
	pub const fn dir(self) -> &'static str {
		match self {
			Self::Body => "body",
			Self::Head => "head",
		}
	}
}

/// Bind-pose mesh a garment is fitted to (art file stem, not species).
///
/// Fitted GLBs live at `characters/clothes/{slot}/{stem}/{garment}.glb`.
/// [`Self::Canonical`] keeps the unfitted catalog file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClothingHost {
	#[default]
	Canonical,
	Body(&'static str),
	Head(&'static str),
}

impl ClothingHost {
	pub const HUMANOID: Self = Self::Body("humanoid_full_body");
	pub const LERON: Self = Self::Body("leron_biped_full_body");
	pub const IGEO: Self = Self::Body("igeo_biped_full_body");
	pub const WUMBUS: Self = Self::Body("wumbus_biped_full_body");
	pub const TUBERWABER: Self = Self::Body("tuberwaber_body");
	pub const CRANE: Self = Self::Body("crane_body");
	pub const WHELP: Self = Self::Body("whelp_bird");
	pub const SPARROW: Self = Self::Body("sparrow_body");
	pub const LIBIRD: Self = Self::Body("libird_body");

	pub const fn body(stem: &'static str) -> Self {
		Self::Body(stem)
	}

	pub const fn head(stem: &'static str) -> Self {
		Self::Head(stem)
	}

	pub const fn slot(self) -> Option<ClothingSlot> {
		match self {
			Self::Canonical => None,
			Self::Body(_) => Some(ClothingSlot::Body),
			Self::Head(_) => Some(ClothingSlot::Head),
		}
	}

	pub const fn stem(self) -> Option<&'static str> {
		match self {
			Self::Canonical => None,
			Self::Body(stem) | Self::Head(stem) => Some(stem),
		}
	}
}

/// Shared clothing catalog; layers compose across species.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClothingMesh {
	TankTop,
	Tunic,
	LongDress,
	ShortDress,
	FittedCoat,
	RobeCoat,
	Robe,
	Hood,
	Pants,
	KneeHighBoots,
	HaremPants,
	HaremPantsUpper,
	HaremPantsLowerWrap,
}

impl ClothingMesh {
	pub const VALUES: &'static [Self] = &[
		Self::TankTop,
		Self::Tunic,
		Self::LongDress,
		Self::ShortDress,
		Self::FittedCoat,
		Self::RobeCoat,
		Self::Robe,
		Self::Hood,
		Self::Pants,
		Self::KneeHighBoots,
		Self::HaremPants,
		Self::HaremPantsUpper,
		Self::HaremPantsLowerWrap,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::TankTop => "tank-top",
			Self::Tunic => "tunic",
			Self::LongDress => "long-dress",
			Self::ShortDress => "short-dress",
			Self::FittedCoat => "fitted-coat",
			Self::RobeCoat => "robe-coat",
			Self::Robe => "robe",
			Self::Hood => "hood",
			Self::Pants => "pants",
			Self::KneeHighBoots => "knee-high-boots",
			Self::HaremPants => "harem-pants",
			Self::HaremPantsUpper => "harem-pants-upper",
			Self::HaremPantsLowerWrap => "harem-pants-lower-wrap",
		}
	}

	pub const fn file_stem(self) -> &'static str {
		match self {
			Self::TankTop => "tank_top",
			Self::Tunic => "tunic",
			Self::LongDress => "long_dress",
			Self::ShortDress => "short_dress",
			Self::FittedCoat => "fitted_coat",
			Self::RobeCoat => "robe_coat",
			Self::Robe => "robe",
			Self::Hood => "hood",
			Self::Pants => "pants",
			Self::KneeHighBoots => "knee_high_boots",
			Self::HaremPants => "harem_pants_unified",
			Self::HaremPantsUpper => "harem_pants_top",
			Self::HaremPantsLowerWrap => "harem_pants_bottom_wrap",
		}
	}

	pub const fn slot(self) -> ClothingSlot {
		match self {
			Self::Hood => ClothingSlot::Head,
			_ => ClothingSlot::Body,
		}
	}

	/// Garments with per-host fitted GLBs under `clothes/{slot}/{host}/`.
	pub const fn uses_host_fit(self) -> bool {
		matches!(self, Self::TankTop | Self::FittedCoat | Self::Robe)
	}

	/// Unfitted catalog path relative to the `maybraid/assets` root.
	pub const fn path(self) -> &'static str {
		match self {
			Self::TankTop => CLOTHING_TANK_TOP,
			Self::Tunic => CLOTHING_TUNIC,
			Self::LongDress => CLOTHING_LONG_DRESS,
			Self::ShortDress => CLOTHING_SHORT_DRESS,
			Self::FittedCoat => CLOTHING_FITTED_COAT,
			Self::RobeCoat => CLOTHING_ROBE_COAT,
			Self::Robe => CLOTHING_ROBE,
			Self::Hood => CLOTHING_HOOD,
			Self::Pants => CLOTHING_PANTS,
			Self::KneeHighBoots => CLOTHING_KNEE_HIGH_BOOTS,
			Self::HaremPants => CLOTHING_HAREM_PANTS,
			Self::HaremPantsUpper => CLOTHING_HAREM_PANTS_UPPER,
			Self::HaremPantsLowerWrap => CLOTHING_HAREM_PANTS_LOWER_WRAP,
		}
	}

	/// Fitted path when this garment has host wraps and `host` matches [`Self::slot`].
	pub fn path_on(self, host: ClothingHost) -> String {
		match (self.slot(), host) {
			(ClothingSlot::Body, ClothingHost::Body(stem)) if self.uses_host_fit() => {
				format!("characters/clothes/body/{stem}/{}.glb", self.file_stem())
			}
			(ClothingSlot::Head, ClothingHost::Head(stem)) if self.uses_host_fit() => {
				format!("characters/clothes/head/{stem}/{}.glb", self.file_stem())
			}
			_ => self.path().to_string(),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tank_top_fits_igeo_body() {
		assert_eq!(
			ClothingMesh::TankTop.path_on(ClothingHost::IGEO),
			"characters/clothes/body/igeo_biped_full_body/tank_top.glb"
		);
	}

	#[test]
	fn fitted_coat_fits_igeo_body() {
		assert_eq!(
			ClothingMesh::FittedCoat.path_on(ClothingHost::IGEO),
			"characters/clothes/body/igeo_biped_full_body/fitted_coat.glb"
		);
	}

	#[test]
	fn tunic_without_host_fits_stays_canonical() {
		assert_eq!(ClothingMesh::Tunic.path_on(ClothingHost::IGEO), ClothingMesh::Tunic.path());
	}

	#[test]
	fn robe_fits_igeo_body() {
		assert_eq!(
			ClothingMesh::Robe.path_on(ClothingHost::IGEO),
			"characters/clothes/body/igeo_biped_full_body/robe.glb"
		);
	}

	#[test]
	fn hood_ignores_body_host() {
		assert_eq!(ClothingMesh::Hood.path_on(ClothingHost::IGEO), ClothingMesh::Hood.path());
	}

	#[test]
	fn canonical_host_is_catalog_path() {
		assert_eq!(
			ClothingMesh::TankTop.path_on(ClothingHost::Canonical),
			ClothingMesh::TankTop.path()
		);
	}
}
