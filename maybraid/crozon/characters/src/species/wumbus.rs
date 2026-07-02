//! Wumbus species definition.
//!
//! Bearlike humanoid: ortho bear head on the Wumbus body, flank ears, canine snout,
//! dark fur colors with lighter contrasting features, and optional harrowed crown.

pub mod assets;
pub mod pose;

use crate::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, EyeMesh, HairMesh},
		SpeciesConfig,
	},
	ResolvedCharacterAssembly,
};

use clap::ValueEnum;

use assets::WumbusAssets;

pub use assets::{WumbusHeadMesh, WumbusHornMesh, WumbusMouthMesh};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusSkinColor {
	#[default]
	Chocolate,
	Espresso,
	Umber,
	Soot,
}

impl WumbusSkinColor {
	pub const VALUES: &'static [Self] =
		&[Self::Chocolate, Self::Espresso, Self::Umber, Self::Soot];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Chocolate => "chocolate",
			Self::Espresso => "espresso",
			Self::Umber => "umber",
			Self::Soot => "soot",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Chocolate => bevy::prelude::Color::srgb(0.28, 0.20, 0.16),
			Self::Espresso => bevy::prelude::Color::srgb(0.18, 0.14, 0.12),
			Self::Umber => bevy::prelude::Color::srgb(0.32, 0.24, 0.18),
			Self::Soot => bevy::prelude::Color::srgb(0.12, 0.11, 0.10),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusEyeColor {
	#[default]
	PaleBlue,
	Honey,
	Sage,
}

impl WumbusEyeColor {
	pub const VALUES: &'static [Self] = &[Self::PaleBlue, Self::Honey, Self::Sage];

	pub const fn label(self) -> &'static str {
		match self {
			Self::PaleBlue => "pale-blue",
			Self::Honey => "honey",
			Self::Sage => "sage",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::PaleBlue => bevy::prelude::Color::srgb(0.62, 0.72, 0.78),
			Self::Honey => bevy::prelude::Color::srgb(0.78, 0.68, 0.42),
			Self::Sage => bevy::prelude::Color::srgb(0.58, 0.68, 0.55),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusEarColor {
	#[default]
	Cream,
	Sandy,
	RustTip,
}

impl WumbusEarColor {
	pub const VALUES: &'static [Self] = &[Self::Cream, Self::Sandy, Self::RustTip];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Cream => "cream",
			Self::Sandy => "sandy",
			Self::RustTip => "rust-tip",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Cream => bevy::prelude::Color::srgb(0.88, 0.82, 0.72),
			Self::Sandy => bevy::prelude::Color::srgb(0.78, 0.68, 0.52),
			Self::RustTip => bevy::prelude::Color::srgb(0.72, 0.48, 0.32),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusMouthColor {
	#[default]
	Blush,
	DustyRose,
	PaleCoral,
}

impl WumbusMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Blush, Self::DustyRose, Self::PaleCoral];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Blush => "blush",
			Self::DustyRose => "dusty-rose",
			Self::PaleCoral => "pale-coral",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Blush => bevy::prelude::Color::srgb(0.82, 0.62, 0.58),
			Self::DustyRose => bevy::prelude::Color::srgb(0.75, 0.55, 0.52),
			Self::PaleCoral => bevy::prelude::Color::srgb(0.88, 0.66, 0.60),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusHornColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl WumbusHornColor {
	pub const VALUES: &'static [Self] = &[Self::Ivory, Self::Wheat, Self::PaleGold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ivory => "ivory",
			Self::Wheat => "wheat",
			Self::PaleGold => "pale-gold",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ivory => bevy::prelude::Color::srgb(0.88, 0.84, 0.74),
			Self::Wheat => bevy::prelude::Color::srgb(0.82, 0.74, 0.56),
			Self::PaleGold => bevy::prelude::Color::srgb(0.90, 0.82, 0.62),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum WumbusSpineColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl WumbusSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Ivory, Self::Wheat, Self::PaleGold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ivory => "ivory",
			Self::Wheat => "wheat",
			Self::PaleGold => "pale-gold",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ivory => bevy::prelude::Color::srgb(0.88, 0.84, 0.74),
			Self::Wheat => bevy::prelude::Color::srgb(0.82, 0.74, 0.56),
			Self::PaleGold => bevy::prelude::Color::srgb(0.90, 0.82, 0.62),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WumbusColors {
	pub skin: WumbusSkinColor,
	pub eyes: WumbusEyeColor,
	pub ears: WumbusEarColor,
	pub mouth: WumbusMouthColor,
	pub horns: WumbusHornColor,
	pub spine: WumbusSpineColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for WumbusColors {
	fn default() -> Self {
		Self {
			skin: WumbusSkinColor::Chocolate,
			eyes: WumbusEyeColor::PaleBlue,
			ears: WumbusEarColor::Cream,
			mouth: WumbusMouthColor::Blush,
			horns: WumbusHornColor::Ivory,
			spine: WumbusSpineColor::Ivory,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl WumbusColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> BraidmanColor {
		self.clothing
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.color)
			.unwrap_or(self.clothing_default)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: BraidmanColor) {
		if let Some(choice) = self.clothing.iter_mut().find(|choice| choice.clothing == clothing) {
			choice.color = color;
		} else {
			self.clothing.push(ClothingColor { clothing, color });
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WumbusConfig {
	pub horns: WumbusHornMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: WumbusColors,
}

impl Default for WumbusConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl WumbusConfig {
	pub fn default_preview() -> Self {
		Self {
			horns: WumbusHornMesh::None,
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: WumbusColors::default(),
		}
	}

	pub fn status_label(&self) -> String {
		let clothing = if self.clothing.is_empty() {
			"none".into()
		} else {
			self.clothing
				.iter()
				.map(|clothing| clothing.label())
				.collect::<Vec<_>>()
				.join(",")
		};
		format!(
			"wumbus horns={} eye={} hair={} clothing={} skin={} eyes={} ears={} horn_color={} hair_color={}",
			self.horns.label(),
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.horns.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for WumbusConfig {
	fn species_name(&self) -> &'static str {
		"wumbus"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		WumbusAssets::resolve(self)
	}
}
