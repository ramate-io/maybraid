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
	Pearl,
	Frost,
	Ivory,
}

impl SpibmomEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::Frost, Self::Ivory];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::Frost => "frost",
			Self::Ivory => "ivory",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.96, 0.96, 0.94),
			Self::Frost => bevy::prelude::Color::srgb(0.94, 0.96, 0.98),
			Self::Ivory => bevy::prelude::Color::srgb(0.96, 0.94, 0.90),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomEarColor {
	#[default]
	Umber,
	Charcoal,
	DeepSlate,
}

impl SpibmomEarColor {
	pub const VALUES: &'static [Self] = &[Self::Umber, Self::Charcoal, Self::DeepSlate];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Umber => "umber",
			Self::Charcoal => "charcoal",
			Self::DeepSlate => "deep-slate",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Umber => bevy::prelude::Color::srgb(0.32, 0.24, 0.18),
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.22, 0.20),
			Self::DeepSlate => bevy::prelude::Color::srgb(0.20, 0.24, 0.28),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomMouthColor {
	#[default]
	Espresso,
	Charcoal,
	DeepSlate,
}

impl SpibmomMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Espresso, Self::Charcoal, Self::DeepSlate];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Espresso => "espresso",
			Self::Charcoal => "charcoal",
			Self::DeepSlate => "deep-slate",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Espresso => bevy::prelude::Color::srgb(0.22, 0.16, 0.14),
			Self::Charcoal => bevy::prelude::Color::srgb(0.20, 0.20, 0.22),
			Self::DeepSlate => bevy::prelude::Color::srgb(0.18, 0.22, 0.26),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomCrownColor {
	#[default]
	Charcoal,
	DeepBronze,
	DarkUmber,
}

impl SpibmomCrownColor {
	pub const VALUES: &'static [Self] = &[Self::Charcoal, Self::DeepBronze, Self::DarkUmber];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Charcoal => "charcoal",
			Self::DeepBronze => "deep-bronze",
			Self::DarkUmber => "dark-umber",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.24, 0.26),
			Self::DeepBronze => bevy::prelude::Color::srgb(0.34, 0.26, 0.18),
			Self::DarkUmber => bevy::prelude::Color::srgb(0.30, 0.22, 0.16),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomSpineColor {
	#[default]
	Charcoal,
	DeepBronze,
	DarkUmber,
}

impl SpibmomSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Charcoal, Self::DeepBronze, Self::DarkUmber];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Charcoal => "charcoal",
			Self::DeepBronze => "deep-bronze",
			Self::DarkUmber => "dark-umber",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Charcoal => bevy::prelude::Color::srgb(0.24, 0.24, 0.26),
			Self::DeepBronze => bevy::prelude::Color::srgb(0.34, 0.26, 0.18),
			Self::DarkUmber => bevy::prelude::Color::srgb(0.30, 0.22, 0.16),
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
			eyes: SpibmomEyeColor::Pearl,
			ears: SpibmomEarColor::Umber,
			mouth: SpibmomMouthColor::Espresso,
			crown: SpibmomCrownColor::Charcoal,
			spine: SpibmomSpineColor::Charcoal,
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
