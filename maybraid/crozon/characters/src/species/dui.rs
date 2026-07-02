//! Dui species definition.
//!
//! Tall slender humanoid: barred bowl head on the Igeo body, thorn horns as eyes,
//! optional t-bar nose, small common mouth, no ears, and soft earth-tone skin colors.

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

use assets::DuiAssets;

pub use assets::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiSkinColor {
	#[default]
	Purple,
	DesertBrown,
	Blue,
	Gold,
}

impl DuiSkinColor {
	pub const VALUES: &'static [Self] =
		&[Self::Purple, Self::DesertBrown, Self::Blue, Self::Gold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Purple => "purple",
			Self::DesertBrown => "desert-brown",
			Self::Blue => "blue",
			Self::Gold => "gold",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Purple => bevy::prelude::Color::srgb(0.54, 0.47, 0.48),
			Self::DesertBrown => bevy::prelude::Color::srgb(0.62, 0.54, 0.44),
			Self::Blue => bevy::prelude::Color::srgb(0.50, 0.54, 0.52),
			Self::Gold => bevy::prelude::Color::srgb(0.66, 0.58, 0.44),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiEyeColor {
	#[default]
	Black,
}

impl DuiEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Black];

	pub const fn label(self) -> &'static str {
		"black"
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiMouthColor {
	#[default]
	Red,
	Blue,
}

impl DuiMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Red, Self::Blue];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Red => "red",
			Self::Blue => "blue",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Red => bevy::prelude::Color::srgb(0.55, 0.36, 0.32),
			Self::Blue => bevy::prelude::Color::srgb(0.40, 0.44, 0.50),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiNoseColor {
	#[default]
	Black,
}

impl DuiNoseColor {
	pub const VALUES: &'static [Self] = &[Self::Black];

	pub const fn label(self) -> &'static str {
		"black"
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuiColors {
	pub skin: DuiSkinColor,
	pub eyes: DuiEyeColor,
	pub nose_color: DuiNoseColor,
	pub mouth: DuiMouthColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for DuiColors {
	fn default() -> Self {
		Self {
			skin: DuiSkinColor::Purple,
			eyes: DuiEyeColor::Black,
			nose_color: DuiNoseColor::Black,
			mouth: DuiMouthColor::Red,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl DuiColors {
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
pub struct DuiConfig {
	pub nose: DuiNoseMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: DuiColors,
}

impl Default for DuiConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl DuiConfig {
	pub fn default_preview() -> Self {
		Self {
			nose: DuiNoseMesh::None,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: DuiColors::default(),
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
			"dui nose={} hair={} clothing={} skin={} hair_color={}",
			self.nose.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for DuiConfig {
	fn species_name(&self) -> &'static str {
		"dui"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		DuiAssets::resolve(self)
	}
}
