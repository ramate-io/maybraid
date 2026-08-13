//! Wumbus species definition.
//!
//! Bearlike humanoid: ortho bear head on the Wumbus body, flank ears, canine snout,
//! dark fur colors with lighter contrasting features, and optional harrowed crown.

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

use assets::WumbusAssets;

pub use assets::{WumbusHeadMesh, WumbusHornMesh, WumbusMouthMesh};
pub use palette::{
	WumbusEarColor, WumbusEyeColor, WumbusHornColor, WumbusMouthColor, WumbusSkinColor,
	WumbusSpineColor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WumbusColors {
	pub skin: WumbusSkinColor,
	pub eyes: WumbusEyeColor,
	pub ears: WumbusEarColor,
	pub mouth: WumbusMouthColor,
	pub horns: WumbusHornColor,
	pub spine: WumbusSpineColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
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
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl WumbusColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
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

	/// Inner recipe plus clothing layers (`Clothed<Wumbus>`).
	pub fn clothed(&self) -> Clothed<crate::species::wumbus::bsn::Wumbus> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for WumbusConfig {
	type Components = crate::species::wumbus::bsn::Wumbus;

	fn components(&self) -> Self::Components {
		crate::species::wumbus::bsn::Wumbus::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
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
