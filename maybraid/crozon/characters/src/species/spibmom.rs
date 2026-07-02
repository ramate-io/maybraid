//! Spibmom species definition.
//!
//! Meerkat-headed Wumbus body with a long neck, snail-back spine, finbone crown,
//! small flank ears, and igny snout. Soft blue skin with light contrasting accents.

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

use assets::SpibmomAssets;

pub use assets::{SpibmomCrownMesh, SpibmomHeadMesh, SpibmomMouthMesh};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomSkinColor {
	#[default]
	PowderBlue,
	MistBlue,
	SoftSky,
	PalePeriwinkle,
	DustyBlue,
}

impl SpibmomSkinColor {
	pub const VALUES: &'static [Self] = &[
		Self::PowderBlue,
		Self::MistBlue,
		Self::SoftSky,
		Self::PalePeriwinkle,
		Self::DustyBlue,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::PowderBlue => "powder-blue",
			Self::MistBlue => "mist-blue",
			Self::SoftSky => "soft-sky",
			Self::PalePeriwinkle => "pale-periwinkle",
			Self::DustyBlue => "dusty-blue",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::PowderBlue => bevy::prelude::Color::srgb(0.72, 0.80, 0.88),
			Self::MistBlue => bevy::prelude::Color::srgb(0.65, 0.74, 0.82),
			Self::SoftSky => bevy::prelude::Color::srgb(0.58, 0.68, 0.78),
			Self::PalePeriwinkle => bevy::prelude::Color::srgb(0.70, 0.72, 0.86),
			Self::DustyBlue => bevy::prelude::Color::srgb(0.55, 0.62, 0.72),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomEyeColor {
	#[default]
	SoftAmber,
	PaleGold,
	WarmBrown,
}

impl SpibmomEyeColor {
	pub const VALUES: &'static [Self] = &[Self::SoftAmber, Self::PaleGold, Self::WarmBrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftAmber => "soft-amber",
			Self::PaleGold => "pale-gold",
			Self::WarmBrown => "warm-brown",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftAmber => bevy::prelude::Color::srgb(0.82, 0.72, 0.48),
			Self::PaleGold => bevy::prelude::Color::srgb(0.88, 0.80, 0.58),
			Self::WarmBrown => bevy::prelude::Color::srgb(0.62, 0.48, 0.36),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomEarColor {
	#[default]
	Cream,
	Sandy,
	PaleRose,
}

impl SpibmomEarColor {
	pub const VALUES: &'static [Self] = &[Self::Cream, Self::Sandy, Self::PaleRose];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Cream => "cream",
			Self::Sandy => "sandy",
			Self::PaleRose => "pale-rose",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Cream => bevy::prelude::Color::srgb(0.90, 0.86, 0.76),
			Self::Sandy => bevy::prelude::Color::srgb(0.82, 0.74, 0.58),
			Self::PaleRose => bevy::prelude::Color::srgb(0.88, 0.74, 0.70),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomMouthColor {
	#[default]
	SoftBlush,
	Buttercream,
	PaleCoral,
}

impl SpibmomMouthColor {
	pub const VALUES: &'static [Self] = &[Self::SoftBlush, Self::Buttercream, Self::PaleCoral];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftBlush => "soft-blush",
			Self::Buttercream => "buttercream",
			Self::PaleCoral => "pale-coral",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftBlush => bevy::prelude::Color::srgb(0.90, 0.78, 0.74),
			Self::Buttercream => bevy::prelude::Color::srgb(0.92, 0.86, 0.70),
			Self::PaleCoral => bevy::prelude::Color::srgb(0.88, 0.66, 0.60),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomCrownColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl SpibmomCrownColor {
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
pub enum SpibmomSpineColor {
	#[default]
	Ivory,
	Wheat,
	PaleGold,
}

impl SpibmomSpineColor {
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
pub struct SpibmomColors {
	pub skin: SpibmomSkinColor,
	pub eyes: SpibmomEyeColor,
	pub ears: SpibmomEarColor,
	pub mouth: SpibmomMouthColor,
	pub crown: SpibmomCrownColor,
	pub spine: SpibmomSpineColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for SpibmomColors {
	fn default() -> Self {
		Self {
			skin: SpibmomSkinColor::PowderBlue,
			eyes: SpibmomEyeColor::SoftAmber,
			ears: SpibmomEarColor::Cream,
			mouth: SpibmomMouthColor::SoftBlush,
			crown: SpibmomCrownColor::Ivory,
			spine: SpibmomSpineColor::Ivory,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl SpibmomColors {
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
pub struct SpibmomConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: SpibmomColors,
}

impl Default for SpibmomConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl SpibmomConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: SpibmomColors::default(),
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
			"spibmom eye={} hair={} clothing={} skin={} eyes={} ears={} mouth={} crown={} spine={} hair_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.mouth.label(),
			self.colors.crown.label(),
			self.colors.spine.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for SpibmomConfig {
	fn species_name(&self) -> &'static str {
		"spibmom"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		SpibmomAssets::resolve(self)
	}
}
