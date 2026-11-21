//! One configurable storybook-tree recipe. Species appearance stays out of WESL.

use vegetation_groves::temperate_lower_massives::{
	TemperateLowerMassivesCell, TemperateLowerMassivesItem, TemperateLowerMassivesStorybook,
};
use vegetation_shaders::wesl::VegetationPalette;

use crate::layout::LeafCloseStyle;

/// Tree silhouette family. Recipes stay in this crate, not shared WESL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeForm {
	Storybook,
	Willow,
}

impl TreeForm {
	pub fn species_id(self) -> f32 {
		match self {
			Self::Storybook => 2.0,
			Self::Willow => 3.0,
		}
	}
}

/// Authored ranges matching Temperate Lower Massives storybook trees.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRecipe {
	pub height_m: std::ops::RangeInclusive<f32>,
	pub limb_count: std::ops::RangeInclusive<u32>,
	pub branch_depth: std::ops::RangeInclusive<u32>,
	pub stalk_height_fraction: f32,
	pub stalk_radius_m: std::ops::RangeInclusive<f32>,
	pub canopy_spread_m: std::ops::RangeInclusive<f32>,
	pub leaf_size_m: std::ops::RangeInclusive<f32>,
	/// `0` leafless skeleton, `1` full layered canopy.
	pub foliage_density: f32,
	pub wood_stiffness: f32,
	pub wood: VegetationPalette,
	pub foliage: VegetationPalette,
	pub form: TreeForm,
	pub drape_count: std::ops::RangeInclusive<u32>,
	pub drape_length_m: std::ops::RangeInclusive<f32>,
	/// Close-up card treatment. Far LOD still reads as solid clumps.
	pub leaf_close_style: LeafCloseStyle,
}

impl TreeRecipe {
	/// Compact adapter onto Temperate Lower Massives storybook ranges.
	pub fn storybook() -> Self {
		Self::from_lower_massive_storybook(1.0)
	}

	pub fn young_storybook() -> Self {
		let mut recipe = Self::storybook();
		recipe.height_m = 8.0..=12.0;
		recipe.limb_count = 5..=7;
		recipe.canopy_spread_m = 3.5..=5.5;
		recipe
	}

	pub fn sparse_storybook() -> Self {
		Self::storybook().with_foliage_density(0.22)
	}

	pub fn leafless_storybook() -> Self {
		Self::storybook().with_foliage_density(0.0)
	}

	pub fn from_lower_massive_storybook(foliage_density: f32) -> Self {
		let TemperateLowerMassivesItem::Storybook(authored) =
			TemperateLowerMassivesCell::LowerMassiveStorybook.item()
		else {
			unreachable!("LowerMassiveStorybook always maps to a storybook item");
		};
		Self::from_storybook(authored, foliage_density)
	}

	pub fn from_storybook(tree: &TemperateLowerMassivesStorybook, foliage_density: f32) -> Self {
		Self {
			height_m: tree.height.start..=tree.height.end,
			limb_count: 6..=9,
			branch_depth: 2..=3,
			stalk_height_fraction: 0.78,
			stalk_radius_m: tree.stalk_radius.start..=tree.stalk_radius.end,
			canopy_spread_m: tree.canopy_spread.start..=tree.canopy_spread.end,
			leaf_size_m: 1.55..=2.55,
			foliage_density: foliage_density.clamp(0.0, 1.0),
			wood_stiffness: 0.82,
			wood: VegetationPalette::botw_wood(),
			foliage: VegetationPalette::botw_canopy_crown(),
			form: TreeForm::Storybook,
			drape_count: 0..=0,
			drape_length_m: 0.0..=0.0,
			leaf_close_style: LeafCloseStyle::Smooth,
		}
	}

	/// Weeping willow: long hanging curtains from the outer crown.
	pub fn willow() -> Self {
		Self {
			height_m: 10.0..=14.0,
			limb_count: 6..=8,
			branch_depth: 2..=2,
			stalk_height_fraction: 0.52,
			stalk_radius_m: 0.22..=0.32,
			canopy_spread_m: 5.5..=8.0,
			leaf_size_m: 1.15..=1.85,
			foliage_density: 1.0,
			wood_stiffness: 0.56,
			wood: VegetationPalette::botw_wood(),
			foliage: VegetationPalette::botw_canopy_crown(),
			form: TreeForm::Willow,
			drape_count: 3..=4,
			drape_length_m: 3.8..=6.4,
			leaf_close_style: LeafCloseStyle::Smooth,
		}
	}

	pub fn with_foliage_density(mut self, density: f32) -> Self {
		self.foliage_density = density.clamp(0.0, 1.0);
		self
	}

	pub fn with_leaf_close_style(mut self, style: LeafCloseStyle) -> Self {
		self.leaf_close_style = style;
		self
	}

	pub fn is_willow(&self) -> bool {
		self.form == TreeForm::Willow
	}
}

impl Default for TreeRecipe {
	fn default() -> Self {
		Self::storybook()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn storybook_matches_lower_massive_authored_ranges() -> anyhow::Result<()> {
		let recipe = TreeRecipe::storybook();
		let TemperateLowerMassivesItem::Storybook(authored) =
			TemperateLowerMassivesCell::LowerMassiveStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(*recipe.height_m.start(), authored.height.start);
		assert_eq!(*recipe.height_m.end(), authored.height.end);
		assert_eq!(*recipe.canopy_spread_m.start(), authored.canopy_spread.start);
		assert_eq!(*recipe.stalk_radius_m.end(), authored.stalk_radius.end);
		assert_eq!(recipe.form, TreeForm::Storybook);
		assert_eq!(recipe.leaf_close_style, LeafCloseStyle::Smooth);
		Ok(())
	}

	#[test]
	fn leaf_close_style_is_per_recipe() -> anyhow::Result<()> {
		let dithered = TreeRecipe::storybook().with_leaf_close_style(LeafCloseStyle::Dither);
		assert_eq!(dithered.leaf_close_style, LeafCloseStyle::Dither);
		assert_eq!(TreeRecipe::willow().leaf_close_style, LeafCloseStyle::Smooth);
		Ok(())
	}

	#[test]
	fn willow_is_a_distinct_recipe() -> anyhow::Result<()> {
		let willow = TreeRecipe::willow();
		assert!(willow.is_willow());
		assert!(*willow.drape_count.end() >= 3);
		assert!(*willow.drape_length_m.end() > 4.0);
		assert!(willow.wood_stiffness < TreeRecipe::storybook().wood_stiffness);
		Ok(())
	}
}
