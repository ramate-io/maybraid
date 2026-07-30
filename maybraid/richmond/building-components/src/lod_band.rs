//! Shared distance / extent banding for domain LOD probes.
//!
//! Partitions and roofs each own factor constants and probe components; they share
//! this mapping from `distance / extent` onto High / Medium / Low / UltraLow.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};

use crate::placed::Placement;

/// Four-way band from a distance factor (used by partition and roof probes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl DistanceLodBand {
	/// `distance / extent` → band using domain-specific thresholds.
	pub fn from_factors(factor: f32, high: f32, medium: f32, low: f32) -> Self {
		if factor <= high {
			Self::High
		} else if factor <= medium {
			Self::Medium
		} else if factor <= low {
			Self::Low
		} else {
			Self::UltraLow
		}
	}

	/// UltraLow and Low share the low mesh until a dedicated ultra-low exists.
	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self {
			Self::High => LodSceneLevel::High,
			Self::Medium => LodSceneLevel::Medium,
			Self::UltraLow | Self::Low => LodSceneLevel::Low,
		}
	}

	pub fn status_vs(self, prev: Self) -> LodSceneStatus {
		let prev_l = prev.to_lod_scene_level();
		let curr_l = self.to_lod_scene_level();
		if prev_l == curr_l {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr_l)
		}
	}
}

/// AABB center and characteristic extent (max axis).
pub fn center_extent_from_aabb(aabb: &Aabb3d) -> (Vec3, f32) {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let size = aabb.max - aabb.min;
	(center, size.x.max(size.y).max(size.z).max(1e-4))
}

/// Max absolute placement scale axis.
pub fn characteristic_extent_abs(placement: &Placement) -> f32 {
	placement
		.scale
		.x
		.abs()
		.max(placement.scale.y.abs())
		.max(placement.scale.z.abs())
		.max(1e-4)
}

/// Mid-height of a placed kit (translation + half Y scale).
pub fn placement_center(placement: &Placement) -> Vec3 {
	placement.translation + Vec3::new(0.0, placement.scale.y.abs() * 0.5, 0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_factors_thresholds() -> anyhow::Result<()> {
		assert_eq!(DistanceLodBand::from_factors(2.5, 2.5, 10.0, 500.0), DistanceLodBand::High);
		assert_eq!(DistanceLodBand::from_factors(10.0, 2.5, 10.0, 500.0), DistanceLodBand::Medium);
		assert_eq!(DistanceLodBand::from_factors(500.0, 2.5, 10.0, 500.0), DistanceLodBand::Low);
		assert_eq!(
			DistanceLodBand::from_factors(501.0, 2.5, 10.0, 500.0),
			DistanceLodBand::UltraLow
		);
		Ok(())
	}
}
