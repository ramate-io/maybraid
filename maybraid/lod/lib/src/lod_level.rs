//! Discrete and quantized presentation LOD levels.

use bevy::prelude::Component;

/// Quantized viewer distance (meters), for [`LodSceneLevel::Distance`].
///
/// Raw `f32` distances are not used for equality; bucket by [`Self::QUANTUM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuantizedDistance(pub u32);

impl QuantizedDistance {
	/// Distance quantum in meters.
	pub const QUANTUM: f32 = 1.0;

	pub fn from_meters(meters: f32) -> Self {
		Self(meters.max(0.0).div_euclid(Self::QUANTUM) as u32)
	}

	pub fn to_meters(self) -> f32 {
		self.0 as f32 * Self::QUANTUM
	}
}

/// Presentation LOD selection for a [`crate::gen::LodScene`] host.
///
/// [`Ord`] follows variant order (UltraLow … High, then Distance / Resolution).
/// Used by [`crate::gen::LodScene::scene_lod_level_from_levels`] defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component, Default)]
pub enum LodSceneLevel {
	UltraLow,
	Low,
	Medium,
	#[default]
	High,
	Distance(QuantizedDistance),
	Resolution(u32),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn distance_quantizes() -> anyhow::Result<()> {
		assert_eq!(QuantizedDistance::from_meters(0.0).0, 0);
		assert_eq!(QuantizedDistance::from_meters(0.9).0, 0);
		assert_eq!(QuantizedDistance::from_meters(1.0).0, 1);
		assert_eq!(QuantizedDistance::from_meters(3.7).0, 3);
		Ok(())
	}
}
