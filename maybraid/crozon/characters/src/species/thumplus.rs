//! Thumplus species definition.
//!
//! Whale on the forelimbed rig (~6 m). Authoring body length is ~2 m, so
//! overall scale is 3× via body-rig asset normalization.

pub mod assets;
pub mod bsn;
pub mod palette;
pub mod pose;

use crate::{
	species::SpeciesConfig, CharacterRecipe, Clothed, ClothingLayer, ResolvedCharacterAssembly,
};

use assets::ThumplusAssets;

pub use palette::ThumplusBodyColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumplusColors {
	pub body: ThumplusBodyColor,
}

impl Default for ThumplusColors {
	fn default() -> Self {
		Self { body: ThumplusBodyColor::Ocean }
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumplusConfig {
	pub colors: ThumplusColors,
}

impl Default for ThumplusConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ThumplusConfig {
	pub fn default_preview() -> Self {
		Self { colors: ThumplusColors::default() }
	}

	pub fn status_label(&self) -> String {
		format!("thumplus body={}", self.colors.body.label())
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}

	/// Inner recipe plus clothing layers (`Clothed<Thumplus>`).
	pub fn clothed(&self) -> Clothed<crate::species::thumplus::bsn::Thumplus> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for ThumplusConfig {
	type Components = crate::species::thumplus::bsn::Thumplus;

	fn components(&self) -> Self::Components {
		crate::species::thumplus::bsn::Thumplus::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}

impl SpeciesConfig for ThumplusConfig {
	fn species_name(&self) -> &'static str {
		"thumplus"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		ThumplusAssets::resolve(self)
	}
}
