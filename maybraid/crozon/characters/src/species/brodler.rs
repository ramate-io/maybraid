//! Brodler species definition.
//!
//! Cartoon humanoid: shared Standard body mesh, Gaunt/Full heads, crown horns,
//! species-owned skin/eye colors, and shared hair/clothing catalogs.

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

use assets::BrodlerAssets;

pub use assets::HornMesh;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrodlerHeadMesh {
	#[default]
	Gaunt,
	Full,
}

impl BrodlerHeadMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Gaunt => "gaunt",
			Self::Full => "full",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		use crate::species::common::{HEAD_FULL, HEAD_GAUNT};
		match self {
			Self::Gaunt => HEAD_GAUNT,
			Self::Full => HEAD_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrodlerSkinColor {
	#[default]
	Red,
	Black,
	Yellow,
}

impl BrodlerSkinColor {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Red => "red",
			Self::Black => "black",
			Self::Yellow => "yellow",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Red => bevy::prelude::Color::srgb(0.78, 0.22, 0.18),
			Self::Black => bevy::prelude::Color::srgb(0.12, 0.10, 0.10),
			Self::Yellow => bevy::prelude::Color::srgb(0.92, 0.82, 0.28),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrodlerEyeColor {
	#[default]
	Red,
	Green,
	Black,
}

impl BrodlerEyeColor {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Red => "red",
			Self::Green => "green",
			Self::Black => "black",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Red => bevy::prelude::Color::srgb(0.82, 0.12, 0.10),
			Self::Green => bevy::prelude::Color::srgb(0.18, 0.62, 0.22),
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrodlerColors {
	pub skin: BrodlerSkinColor,
	pub eyes: BrodlerEyeColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for BrodlerColors {
	fn default() -> Self {
		Self {
			skin: BrodlerSkinColor::Red,
			eyes: BrodlerEyeColor::Red,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl BrodlerColors {
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
pub struct BrodlerConfig {
	pub head: BrodlerHeadMesh,
	pub horns: HornMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: BrodlerColors,
}

impl Default for BrodlerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BrodlerConfig {
	pub fn default_preview() -> Self {
		Self {
			head: BrodlerHeadMesh::Gaunt,
			horns: HornMesh::HarrowedCrown,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: BrodlerColors::default(),
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
			"brodler head={} horns={} hair={} clothing={} skin={} eyes={} hair_color={}",
			self.head.label(),
			self.horns.label(),
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

impl SpeciesConfig for BrodlerConfig {
	fn species_name(&self) -> &'static str {
		"brodler"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		BrodlerAssets::resolve(self)
	}
}
