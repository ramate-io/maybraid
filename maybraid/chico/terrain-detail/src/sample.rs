//! Height sampling for rock sit / embed ([RFC-170 §3.1.5]).
//!
//! Same job as grove [`GroveWorldSample::height_at`]: world metres at XZ.
//! Slope reject waits until piles look right on flat ground.

use bevy_math::Vec3;

/// Terrain height field used when placing rocks.
pub trait TerrainDetailWorldSample {
	/// Surface height in world metres at `position` (XZ).
	fn height_at(&self, position: Vec3) -> f32;
}

/// Uniform height for tests and isolated `/show` pins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatTerrainDetailSample {
	pub elevation: f32,
}

impl Default for FlatTerrainDetailSample {
	fn default() -> Self {
		Self { elevation: 0.0 }
	}
}

impl TerrainDetailWorldSample for FlatTerrainDetailSample {
	fn height_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}
}

impl<F> TerrainDetailWorldSample for F
where
	F: Fn(Vec3) -> f32,
{
	fn height_at(&self, position: Vec3) -> f32 {
		self(position)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn flat_sample_is_constant() -> Result<()> {
		let sample = FlatTerrainDetailSample { elevation: 12.5 };
		assert!((sample.height_at(Vec3::new(40.0, 0.0, -8.0)) - 12.5).abs() < 1e-5);
		Ok(())
	}
}
