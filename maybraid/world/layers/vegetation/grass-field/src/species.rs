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
	/// Lush painterly meadow blade, BotW-like: ~58 cm, soft taper, gentle bow.
	pub fn meadow() -> Self {
		Self {
			height_m: 0.58,
			width_m: 0.062,
			bend: 0.32,
			taper: 0.82,
			stiffness: 0.05,
			height_jitter: 0.18,
			palette: VegetationPalette::botw_meadow(),
		}
	}

	/// Slightly taller, cooler shade grass for slopes.
	pub fn shade() -> Self {
		Self {
			height_m: 0.70,
			width_m: 0.052,
			bend: 0.38,
			taper: 0.88,
			stiffness: 0.08,
			height_jitter: 0.22,
			palette: VegetationPalette {
				root_rgb: Vec3::new(0.05, 0.24, 0.05),
				tip_rgb: Vec3::new(0.32, 0.62, 0.16),
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
