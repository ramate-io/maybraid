//! Grass species recipes. Appearance stays here; deformation math stays in WESL.

use bevy::math::Vec3;
use vegetation_shaders::wesl::VegetationPalette;

/// Moderate-height field species. Heights sit between POM turf (~9.5 cm) and
/// authored tuft patches (~1.15 m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrassSpecies {
	pub height_m: f32,
	pub width_m: f32,
	pub bend: f32,
	pub taper: f32,
	pub stiffness: f32,
	pub height_jitter: f32,
	pub palette: VegetationPalette,
}

impl GrassSpecies {
	/// Lush painterly meadow blade, BotW-like: ~58 cm, arched rest pose, soft taper.
	pub fn meadow() -> Self {
		Self {
			height_m: 0.58,
			width_m: 0.078,
			bend: 0.78,
			taper: 0.72,
			stiffness: 0.05,
			height_jitter: 0.22,
			palette: VegetationPalette::botw_meadow(),
		}
	}

	/// Slightly taller, cooler shade grass for slopes.
	pub fn shade() -> Self {
		Self {
			height_m: 0.70,
			width_m: 0.068,
			bend: 0.92,
			taper: 0.78,
			stiffness: 0.08,
			height_jitter: 0.26,
			palette: VegetationPalette {
				root_rgb: Vec3::new(0.04, 0.18, 0.07),
				tip_rgb: Vec3::new(0.28, 0.54, 0.16),
			},
		}
	}

	pub fn with_coverage_height(mut self, height_m: f32) -> Self {
		self.height_m = height_m.max(0.05);
		self
	}
}

impl Default for GrassSpecies {
	fn default() -> Self {
		Self::meadow()
	}
}
