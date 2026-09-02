//! Persistable create-mode appearance. Clothing lives on the inventory bag.

use serde::{Deserialize, Serialize};

use crate::species::{
	braidman::BraidmanConfig, brodler::BrodlerConfig, dui::DuiConfig, lero::LeroConfig,
	mygr::MygrConfig, spibmom::SpibmomConfig, tuberwaber::TuberwaberConfig, wumbus::WumbusConfig,
};

/// Humanoid appearance written to `characters/{id}.json`. Worn garments are not
/// stored here; they come from the matching inventory file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "species", content = "config", rename_all = "kebab-case")]
pub enum CharacterAppearance {
	Braidman(BraidmanConfig),
	Brodler(BrodlerConfig),
	Mygr(MygrConfig),
	Dui(DuiConfig),
	Wumbus(WumbusConfig),
	Lero(LeroConfig),
	Spibmom(SpibmomConfig),
	Tuberwaber(TuberwaberConfig),
}

impl Default for CharacterAppearance {
	fn default() -> Self {
		Self::Braidman(BraidmanConfig::default_preview())
	}
}

impl CharacterAppearance {
	pub fn species_id(&self) -> &'static str {
		match self {
			Self::Braidman(_) => "braidman",
			Self::Brodler(_) => "brodler",
			Self::Mygr(_) => "mygr",
			Self::Dui(_) => "dui",
			Self::Wumbus(_) => "wumbus",
			Self::Lero(_) => "lero",
			Self::Spibmom(_) => "spibmom",
			Self::Tuberwaber(_) => "tuberwaber",
		}
	}

	pub fn species_title(&self) -> &'static str {
		match self {
			Self::Braidman(_) => "Braidman",
			Self::Brodler(_) => "Brodler",
			Self::Mygr(_) => "Mygr",
			Self::Dui(_) => "Dui",
			Self::Wumbus(_) => "Wumbus",
			Self::Lero(_) => "Lero",
			Self::Spibmom(_) => "Spibmom",
			Self::Tuberwaber(_) => "Tuberwaber",
		}
	}

	/// Drop clothing layers so the model file does not duplicate the bag.
	pub fn strip_clothing(&mut self) {
		match self {
			Self::Braidman(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Brodler(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Mygr(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Dui(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Wumbus(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Lero(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Spibmom(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
			Self::Tuberwaber(config) => strip_humanoid_clothing(
				&mut config.clothing,
				&mut config.colors.clothing,
				&mut config.colors.clothing_materials,
			),
		}
	}
}

fn strip_humanoid_clothing(
	clothing: &mut Vec<crozon_character_items::ClothingMesh>,
	colors: &mut Vec<crozon_character_items::ClothingColor>,
	materials: &mut Vec<crozon_character_items::ClothingMaterialChoice>,
) {
	clothing.clear();
	colors.clear();
	materials.clear();
}
