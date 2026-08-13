//! Dui species definition.
//!
//! Tall slender humanoid: barred bowl head on the Igeo body, thorn horns as eyes,
//! optional t-bar nose, small common mouth, no ears, and soft earth-tone skin colors.

pub mod assets;
pub mod bsn;
pub mod palette;
pub mod pose;

use crate::{
	species::{common::HairMesh, SpeciesConfig},
	CharacterRecipe, Clothed, ClothingLayer, ResolvedCharacterAssembly,
};

use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};

use assets::DuiAssets;

pub use assets::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh};
pub use palette::{DuiEyeColor, DuiMouthColor, DuiNoseColor, DuiSkinColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuiColors {
	pub skin: DuiSkinColor,
	pub eyes: DuiEyeColor,
	pub nose_color: DuiNoseColor,
	pub mouth: DuiMouthColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for DuiColors {
	fn default() -> Self {
		Self {
			skin: DuiSkinColor::Purple,
			eyes: DuiEyeColor::Black,
			nose_color: DuiNoseColor::Black,
			mouth: DuiMouthColor::Red,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl DuiColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
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

	/// Inner recipe plus clothing layers (`Clothed<Dui>`).
	pub fn clothed(&self) -> Clothed<crate::species::dui::bsn::Dui> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for DuiConfig {
	type Components = crate::species::dui::bsn::Dui;

	fn components(&self) -> Self::Components {
		crate::species::dui::bsn::Dui::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
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
