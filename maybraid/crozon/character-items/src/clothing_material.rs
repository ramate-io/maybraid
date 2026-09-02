//! Clothing surface recipes resolved by Crozon’s [`material_ref`] lib.

use clap::ValueEnum;

/// Named clothing looks. Palette[0] is the user color; the recipe is the shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClothingMaterial {
	SpaceSuit,
	Tattered,
	Hawaiian,
	#[default]
	Cloth,
	Scales,
	WizardsVeins,
	Glitter,
}

impl ClothingMaterial {
	pub const VALUES: &'static [Self] = &[
		Self::SpaceSuit,
		Self::Tattered,
		Self::Hawaiian,
		Self::Cloth,
		Self::Scales,
		Self::WizardsVeins,
		Self::Glitter,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SpaceSuit => "space-suit",
			Self::Tattered => "tattered",
			Self::Hawaiian => "hawaiian",
			Self::Cloth => "cloth",
			Self::Scales => "scales",
			Self::WizardsVeins => "wizards-veins",
			Self::Glitter => "glitter",
		}
	}

	/// [`material_ref::MaterialId::Name`] consumed by the Crozon clothing lib.
	pub const fn recipe_id(self) -> &'static str {
		match self {
			Self::SpaceSuit => "clothing_space_suit",
			Self::Tattered => "clothing_tattered",
			Self::Hawaiian => "clothing_hawaiian",
			Self::Cloth => "clothing_cloth",
			Self::Scales => "clothing_scales",
			Self::WizardsVeins => "clothing_wizards_veins",
			Self::Glitter => "clothing_glitter",
		}
	}

	pub fn is_clothing_recipe(name: &str) -> bool {
		matches!(
			name,
			"clothing_space_suit"
				| "clothing_tattered"
				| "clothing_hawaiian"
				| "clothing_cloth"
				| "clothing_scales"
				| "clothing_wizards_veins"
				| "clothing_glitter"
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_look_has_a_recipe_id() {
		for material in ClothingMaterial::VALUES {
			assert!(ClothingMaterial::is_clothing_recipe(material.recipe_id()));
			assert!(!material.label().is_empty());
		}
	}
}
