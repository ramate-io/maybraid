//! Tapp species definition.
//!
//! Thin Topple sibling (~2 ft): whelp body, cartoon meerkat head, long selectable
//! beak (default Sharp), cooler pastel plumage. Overall size via body-rig asset
//! normalization (~0.30×); thin proportions via BraidmanSliders.

pub mod assets;
pub mod bsn;
pub mod palette;
pub mod pose;

use crate::{
	species::{
		common::{EyeMesh, HairMesh},
		SpeciesConfig,
	},
	CharacterRecipe, Clothed, ClothingLayer, ResolvedCharacterAssembly,
};

use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};

use assets::TappAssets;

pub use assets::{TappBeakMesh, TappHeadMesh};
pub use palette::{TappBeakColor, TappEyeColor, TappPlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TappColors {
	pub plumage: TappPlumageColor,
	pub eyes: TappEyeColor,
	pub beak: TappBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for TappColors {
	fn default() -> Self {
		Self {
			plumage: TappPlumageColor::Mist,
			eyes: TappEyeColor::SoftBlue,
			beak: TappBeakColor::Slate,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl TappColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TappConfig {
	pub beak: TappBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: TappColors,
}

impl Default for TappConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl TappConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: TappBeakMesh::Sharp,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: TappColors::default(),
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
			"tapp beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
			self.beak.label(),
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.plumage.label(),
			self.colors.eyes.label(),
			self.colors.beak.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}

	/// Inner recipe plus clothing layers (`Clothed<Tapp>`).
	pub fn clothed(&self) -> Clothed<crate::species::tapp::bsn::Tapp> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for TappConfig {
	type Components = crate::species::tapp::bsn::Tapp;

	fn components(&self) -> Self::Components {
		crate::species::tapp::bsn::Tapp::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
	}
}

impl SpeciesConfig for TappConfig {
	fn species_name(&self) -> &'static str {
		"tapp"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		TappAssets::resolve(self)
	}
}
