//! One configurable small-shrub recipe. Species appearance stays out of WESL.

use vegetation_groves::bush_scrub::BushScrubBush;
use vegetation_shaders::wesl::VegetationPalette;

/// Authored ranges matching Bush Scrub small / sapling bushes.
#[derive(Clone, Debug, PartialEq)]
pub struct BushRecipe {
	pub height_m: std::ops::RangeInclusive<f32>,
	pub shoot_count: std::ops::RangeInclusive<u32>,
	pub branch_depth: std::ops::RangeInclusive<u32>,
	pub radial_strength: std::ops::RangeInclusive<f32>,
	pub vertical_bias: std::ops::RangeInclusive<f32>,
	pub leaf_radius_m: std::ops::RangeInclusive<f32>,
	/// `0` leafless wood inspection, `1` full crown.
	pub foliage_density: f32,
	pub wood_stiffness: f32,
	pub wood: VegetationPalette,
	pub foliage: VegetationPalette,
}

impl BushRecipe {
	/// Compact adapter onto [`BushScrubBush::small`].
	pub fn small_scrub() -> Self {
		Self::from_scrub_bush(BushScrubBush::small(), 1.0)
	}

	pub fn sapling_scrub() -> Self {
		Self::from_scrub_bush(BushScrubBush::sapling(), 1.0)
	}

	pub fn sparse_scrub() -> Self {
		let mut recipe = Self::small_scrub();
		recipe.foliage_density = 0.18;
		recipe
	}

	pub fn leafless_scrub() -> Self {
		let mut recipe = Self::small_scrub();
		recipe.foliage_density = 0.0;
		recipe
	}

	pub fn from_scrub_bush(bush: BushScrubBush, foliage_density: f32) -> Self {
		Self {
			height_m: bush.height.start..=bush.height.end,
			shoot_count: bush.shoot_count,
			branch_depth: bush.branch_depth,
			radial_strength: bush.radial_strength.start..=bush.radial_strength.end,
			vertical_bias: bush.vertical_bias.start..=bush.vertical_bias.end,
			leaf_radius_m: bush.leaf_radius.start..=bush.leaf_radius.end,
			foliage_density: foliage_density.clamp(0.0, 1.0),
			wood_stiffness: 0.72,
			wood: VegetationPalette::botw_wood(),
			foliage: VegetationPalette::botw_scrub_crown(),
		}
	}

	pub fn mid_height_m(&self) -> f32 {
		0.5 * (*self.height_m.start() + *self.height_m.end())
	}

	pub fn mid_leaf_radius_m(&self) -> f32 {
		0.5 * (*self.leaf_radius_m.start() + *self.leaf_radius_m.end())
	}

	/// Collapse height and leaf radius to their midpoints so site scales map 1:1 onto size.
	pub fn pin_authored_size(mut self) -> Self {
		let height = self.mid_height_m();
		let radius = self.mid_leaf_radius_m();
		self.height_m = height..=height;
		self.leaf_radius_m = radius..=radius;
		self
	}

	pub fn with_foliage_density(mut self, density: f32) -> Self {
		self.foliage_density = density.clamp(0.0, 1.0);
		self
	}
}

impl Default for BushRecipe {
	fn default() -> Self {
		Self::small_scrub()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn small_scrub_matches_bush_scrub_authored_ranges() -> anyhow::Result<()> {
		let recipe = BushRecipe::small_scrub();
		let authored = BushScrubBush::small();
		assert_eq!(*recipe.height_m.start(), authored.height.start);
		assert_eq!(*recipe.height_m.end(), authored.height.end);
		assert_eq!(recipe.shoot_count, authored.shoot_count);
		assert_eq!(recipe.branch_depth, authored.branch_depth);
		Ok(())
	}

	#[test]
	fn pin_authored_size_collapses_height_and_leaf_radius_to_mid() -> anyhow::Result<()> {
		let recipe = BushRecipe::small_scrub();
		let pinned = recipe.clone().pin_authored_size();
		assert_eq!(*pinned.height_m.start(), recipe.mid_height_m());
		assert_eq!(*pinned.height_m.end(), recipe.mid_height_m());
		assert_eq!(*pinned.leaf_radius_m.start(), recipe.mid_leaf_radius_m());
		assert_eq!(*pinned.leaf_radius_m.end(), recipe.mid_leaf_radius_m());
		Ok(())
	}
}
