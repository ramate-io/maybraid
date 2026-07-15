//! Chupri species definition.
//!
//! Tiny bipedal bird sibling of Lidder: crane body on the humanoid rig at authored
//! proportions, meerkat head, beak in the mouth slot, plumage-tinted featherhawk.
//! Overall size is about one foot via body-rig asset normalization (~0.15× the
//! ~2 m biped), not per-bone root scale (pelvis/buttocks are root siblings).

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

use assets::ChupriAssets;

pub use assets::{ChupriBeakMesh, ChupriHeadMesh};
pub use palette::{ChupriBeakColor, ChupriEyeColor, ChupriPlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChupriColors {
	pub plumage: ChupriPlumageColor,
	pub eyes: ChupriEyeColor,
	pub beak: ChupriBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for ChupriColors {
	fn default() -> Self {
		Self {
			plumage: ChupriPlumageColor::Magenta,
			eyes: ChupriEyeColor::Turquoise,
			beak: ChupriBeakColor::Tangerine,
			// Crest tint follows plumage; this field is only for shared hair menus.
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl ChupriColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChupriConfig {
	pub beak: ChupriBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: ChupriColors,
}

impl Default for ChupriConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ChupriConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: ChupriBeakMesh::Beak,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: ChupriColors::default(),
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
			"chupri beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
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

impl SpeciesConfig for ChupriConfig {
	fn species_name(&self) -> &'static str {
		"chupri"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		ChupriAssets::resolve(self)
	}
}
