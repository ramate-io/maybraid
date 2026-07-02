//! Lero species definition.
//!
//! Reptilian humanoid: ortho tee head on the Leron body, lerodon tail and spine,
//! lerodon or robrek snout, faded green and red scales, and light accent colors.

pub mod assets;
pub mod pose;

use crate::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, HairMesh},
		SpeciesConfig,
	},
	ResolvedCharacterAssembly,
};

use clap::ValueEnum;

use assets::LeroAssets;

pub use assets::{LeroHeadMesh, LeroMouthMesh};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroSkinColor {
	#[default]
	FadedGreen,
	MossDrift,
	DustySage,
	FadedRed,
	WeatheredRose,
	ClayRust,
}

impl LeroSkinColor {
	pub const VALUES: &'static [Self] = &[
		Self::FadedGreen,
		Self::MossDrift,
		Self::DustySage,
		Self::FadedRed,
		Self::WeatheredRose,
		Self::ClayRust,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::FadedGreen => "faded-green",
			Self::MossDrift => "moss-drift",
			Self::DustySage => "dusty-sage",
			Self::FadedRed => "faded-red",
			Self::WeatheredRose => "weathered-rose",
			Self::ClayRust => "clay-rust",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::FadedGreen => bevy::prelude::Color::srgb(0.52, 0.58, 0.48),
			Self::MossDrift => bevy::prelude::Color::srgb(0.45, 0.52, 0.42),
			Self::DustySage => bevy::prelude::Color::srgb(0.58, 0.62, 0.52),
			Self::FadedRed => bevy::prelude::Color::srgb(0.58, 0.42, 0.40),
			Self::WeatheredRose => bevy::prelude::Color::srgb(0.62, 0.48, 0.46),
			Self::ClayRust => bevy::prelude::Color::srgb(0.52, 0.38, 0.34),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroEyeColor {
	#[default]
	Gold,
	Amber,
	PaleYellow,
}

impl LeroEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Gold, Self::Amber, Self::PaleYellow];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Gold => "gold",
			Self::Amber => "amber",
			Self::PaleYellow => "pale-yellow",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Gold => bevy::prelude::Color::srgb(0.82, 0.72, 0.38),
			Self::Amber => bevy::prelude::Color::srgb(0.78, 0.62, 0.28),
			Self::PaleYellow => bevy::prelude::Color::srgb(0.88, 0.82, 0.55),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroTailColor {
	#[default]
	Pearl,
	PaleIvory,
	Sand,
	PaleMint,
}

impl LeroTailColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::PaleIvory, Self::Sand, Self::PaleMint];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::PaleIvory => "pale-ivory",
			Self::Sand => "sand",
			Self::PaleMint => "pale-mint",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.90, 0.88, 0.82),
			Self::PaleIvory => bevy::prelude::Color::srgb(0.88, 0.84, 0.76),
			Self::Sand => bevy::prelude::Color::srgb(0.82, 0.74, 0.60),
			Self::PaleMint => bevy::prelude::Color::srgb(0.78, 0.88, 0.82),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LeroSpineColor {
	#[default]
	Pearl,
	PaleIvory,
	Sand,
	PaleMint,
}

impl LeroSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::PaleIvory, Self::Sand, Self::PaleMint];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::PaleIvory => "pale-ivory",
			Self::Sand => "sand",
			Self::PaleMint => "pale-mint",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.90, 0.88, 0.82),
			Self::PaleIvory => bevy::prelude::Color::srgb(0.88, 0.84, 0.76),
			Self::Sand => bevy::prelude::Color::srgb(0.82, 0.74, 0.60),
			Self::PaleMint => bevy::prelude::Color::srgb(0.78, 0.88, 0.82),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeroColors {
	pub skin: LeroSkinColor,
	pub eyes: LeroEyeColor,
	pub tail: LeroTailColor,
	pub spine: LeroSpineColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for LeroColors {
	fn default() -> Self {
		Self {
			skin: LeroSkinColor::FadedGreen,
			eyes: LeroEyeColor::Gold,
			tail: LeroTailColor::Pearl,
			spine: LeroSpineColor::Pearl,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl LeroColors {
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
pub struct LeroConfig {
	pub mouth: LeroMouthMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: LeroColors,
}

impl Default for LeroConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl LeroConfig {
	pub fn default_preview() -> Self {
		Self {
			mouth: LeroMouthMesh::Lerodon,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: LeroColors::default(),
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
			"lero mouth={} hair={} clothing={} skin={} eyes={} tail={} spine={} hair_color={}",
			self.mouth.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.tail.label(),
			self.colors.spine.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for LeroConfig {
	fn species_name(&self) -> &'static str {
		"lero"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		LeroAssets::resolve(self)
	}
}
