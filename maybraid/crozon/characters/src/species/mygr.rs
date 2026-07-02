//! Mygr species definition.
//!
//! Catlike humanoid: ortho bear head on the Leron full body, canine snout, flank
//! ears, species-owned fur/eye colors, and shared hair/clothing catalogs.

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

use assets::MygrAssets;

pub use assets::{MygrHeadMesh, MygrMouthMesh};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum MygrSkinColor {
	#[default]
	Ginger,
	Charcoal,
	Silver,
	Cream,
	Tawny,
}

impl MygrSkinColor {
	pub const VALUES: &'static [Self] =
		&[Self::Ginger, Self::Charcoal, Self::Silver, Self::Cream, Self::Tawny];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ginger => "ginger",
			Self::Charcoal => "charcoal",
			Self::Silver => "silver",
			Self::Cream => "cream",
			Self::Tawny => "tawny",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ginger => bevy::prelude::Color::srgb(0.77, 0.48, 0.23),
			Self::Charcoal => bevy::prelude::Color::srgb(0.16, 0.15, 0.14),
			Self::Silver => bevy::prelude::Color::srgb(0.54, 0.56, 0.58),
			Self::Cream => bevy::prelude::Color::srgb(0.91, 0.86, 0.78),
			Self::Tawny => bevy::prelude::Color::srgb(0.55, 0.37, 0.24),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum MygrEyeColor {
	#[default]
	Green,
	Amber,
	Blue,
}

impl MygrEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Green, Self::Amber, Self::Blue];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Green => "green",
			Self::Amber => "amber",
			Self::Blue => "blue",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Green => bevy::prelude::Color::srgb(0.29, 0.55, 0.31),
			Self::Amber => bevy::prelude::Color::srgb(0.79, 0.64, 0.15),
			Self::Blue => bevy::prelude::Color::srgb(0.42, 0.64, 0.82),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MygrColors {
	pub skin: MygrSkinColor,
	pub eyes: MygrEyeColor,
	pub mouth: BraidmanColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for MygrColors {
	fn default() -> Self {
		Self {
			skin: MygrSkinColor::Ginger,
			eyes: MygrEyeColor::Green,
			mouth: BraidmanColor::Natural,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl MygrColors {
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
pub struct MygrConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: MygrColors,
}

impl Default for MygrConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl MygrConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: MygrColors::default(),
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
			"mygr eye={} hair={} clothing={} skin={} eyes={} hair_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for MygrConfig {
	fn species_name(&self) -> &'static str {
		"mygr"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		MygrAssets::resolve(self)
	}
}
