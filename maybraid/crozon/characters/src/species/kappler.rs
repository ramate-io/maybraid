//! Kappler species definition.
//!
//! ~1 m Topple sibling: whelp body, cartoon meerkat head, selectable beak, soft
//! pastel plumage, very short legs. Overall size via body-rig asset
//! normalization (~0.50×).

pub mod assets;
pub mod bsn;
pub mod palette;
pub mod pose;

use crate::{
	species::{
		common::{EyeMesh, HairMesh},
		SpeciesConfig,
	},
	ResolvedCharacterAssembly,
};

use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};

use assets::KapplerAssets;

pub use assets::{KapplerBeakMesh, KapplerHeadMesh};
pub use palette::{KapplerBeakColor, KapplerEyeColor, KapplerPlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapplerColors {
	pub plumage: KapplerPlumageColor,
	pub eyes: KapplerEyeColor,
	pub beak: KapplerBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for KapplerColors {
	fn default() -> Self {
		Self {
			plumage: KapplerPlumageColor::Cream,
			eyes: KapplerEyeColor::SoftAmber,
			beak: KapplerBeakColor::Peach,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl KapplerColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapplerConfig {
	pub beak: KapplerBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: KapplerColors,
}

impl Default for KapplerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl KapplerConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: KapplerBeakMesh::Beak,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: KapplerColors::default(),
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
			"kappler beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
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
}

impl SpeciesConfig for KapplerConfig {
	fn species_name(&self) -> &'static str {
		"kappler"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		KapplerAssets::resolve(self)
	}
}
