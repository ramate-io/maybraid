//! Tipple species definition.
//!
//! Tiny tweetie bird (~1 ft): whelp body on the humanoid rig with meerkat head,
//! fixed hook beak in the mouth slot, and bright abrasive plumage. Overall size
//! uses body-rig asset normalization (~0.15×), not per-bone root scale.

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

use assets::TippleAssets;

pub use assets::{TippleBeakMesh, TippleHeadMesh};
pub use palette::{TippleBeakColor, TippleEyeColor, TipplePlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TippleColors {
	pub plumage: TipplePlumageColor,
	pub eyes: TippleEyeColor,
	pub beak: TippleBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for TippleColors {
	fn default() -> Self {
		Self {
			plumage: TipplePlumageColor::Yellow,
			eyes: TippleEyeColor::Sky,
			beak: TippleBeakColor::Orange,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl TippleColors {
	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TippleConfig {
	pub beak: TippleBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: TippleColors,
}

impl Default for TippleConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl TippleConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: TippleBeakMesh::Hook,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: TippleColors::default(),
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
			"tipple beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
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

	/// Inner recipe plus clothing layers (`Clothed<Tipple>`).
	pub fn clothed(&self) -> Clothed<crate::species::tipple::bsn::Tipple> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for TippleConfig {
	type Components = crate::species::tipple::bsn::Tipple;

	fn components(&self) -> Self::Components {
		crate::species::tipple::bsn::Tipple::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
	}
}

impl SpeciesConfig for TippleConfig {
	fn species_name(&self) -> &'static str {
		"tipple"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		TippleAssets::resolve(self)
	}
}
