//! Height operators produced by Jersey stamps.

pub mod affine;
pub mod grading;

pub use affine::RegionAffineModulation;
pub use grading::RegionGradingModulation;

/// All Jersey height operators — preferred consumer surface for durham compose.
#[derive(Debug, Clone)]
pub enum JerseyModulation {
	Affine(RegionAffineModulation),
	Grading(RegionGradingModulation),
}

impl JerseyModulation {
	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Affine(m) => m.modify_elevation(elevation, x, z),
			Self::Grading(m) => m.modify_elevation(elevation, x, z),
		}
	}
}
