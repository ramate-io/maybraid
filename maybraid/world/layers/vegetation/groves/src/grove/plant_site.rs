//! Compact world-space plant roots for GPU field recomposition.

use bevy::prelude::Vec3;
use vegetation_components::Placement;

/// One grown plant as a GPU field sample: existing root plus authored height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrovePlantSite {
	pub root: Vec3,
	pub height_m: f32,
}

impl GrovePlantSite {
	pub fn from_placement(placement: Placement) -> Self {
		Self { root: placement.translation, height_m: placement.scale.max_element().max(1e-4) }
	}

	pub fn from_root_height(root: Vec3, height_m: f32) -> Self {
		Self { root, height_m: height_m.max(1e-4) }
	}

	/// Scale a GPU recipe whose height band is centered on `recipe_height_m`.
	pub fn recipe_scale(self, recipe_height_m: f32) -> f32 {
		(self.height_m / recipe_height_m.max(1e-4)).max(1e-4)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn recipe_scale_is_height_over_recipe_mid() {
		let site = GrovePlantSite::from_root_height(Vec3::new(1.0, 2.0, 3.0), 28.0);
		assert!((site.recipe_scale(14.0) - 2.0).abs() < 1e-4);
	}
}
