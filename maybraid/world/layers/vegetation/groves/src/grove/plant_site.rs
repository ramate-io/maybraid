//! Compact world-space plant roots for GPU field recomposition.

use bevy::prelude::Vec3;
use vegetation_components::Placement;

/// One grown plant as a GPU field sample: existing root, surface normal, and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrovePlantSite {
	pub root: Vec3,
	pub up: Vec3,
	pub height_m: f32,
}

impl GrovePlantSite {
	pub fn from_placement(placement: Placement) -> Self {
		Self {
			root: placement.translation,
			up: (placement.rotation() * Vec3::Y).normalize_or(Vec3::Y),
			height_m: placement.scale.max_element().max(1e-4),
		}
	}

	pub fn from_root_height(root: Vec3, height_m: f32) -> Self {
		Self { root, up: Vec3::Y, height_m: height_m.max(1e-4) }
	}

	pub fn with_height(mut self, height_m: f32) -> Self {
		self.height_m = height_m.max(1e-4);
		self
	}

	/// Scale a GPU recipe whose height band is centered on `recipe_height_m`.
	pub fn recipe_scale(self, recipe_height_m: f32) -> f32 {
		(self.height_m / recipe_height_m.max(1e-4)).max(1e-4)
	}

	pub fn oriented_height(self) -> (Vec3, Vec3, f32) {
		(self.root, self.up, self.height_m)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use vegetation_components::Placement;

	#[test]
	fn recipe_scale_is_height_over_recipe_mid() {
		let site = GrovePlantSite::from_root_height(Vec3::new(1.0, 2.0, 3.0), 28.0);
		assert!((site.recipe_scale(14.0) - 2.0).abs() < 1e-4);
	}

	#[test]
	fn from_placement_keeps_root_size_and_up() {
		let placement = Placement::new(Vec3::new(4.0, 1.0, -2.0), 0.0)
			.with_scale(Vec3::splat(12.0))
			.with_pitch(std::f32::consts::FRAC_PI_2);
		let site = GrovePlantSite::from_placement(placement);
		assert_eq!(site.root, placement.translation);
		assert!((site.height_m - 12.0).abs() < 1e-4);
		assert!((site.up - Vec3::Z).length() < 1e-4 || (site.up + Vec3::Z).length() < 1e-4);
	}
}
