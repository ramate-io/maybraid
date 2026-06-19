//! Leeward — moderate-density sheltered upper-canopy grove
//! ([RFC-183 §3.4.7.17], [#339](https://github.com/ramate-io/maybraid/issues/339)).
//!
//! Temperate Conifer and Storybook Tree forms on mild lee slopes. Forest-layer attachment remains a
//! follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{Leeward, LeewardStd};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Leeward grove definition.
///
/// Cell footprint sits at the RFC midpoint (`19.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<LeewardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(19.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-19.0, 19.0),
		),
		distribution: LeewardCell::distribution(),
	}
}

/// Ordered leeward varietals ([RFC-183 §3.4.7.17]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeewardCell {
	ShelteredTemperateConifer,
	WindbreakTemperateConifer,
	RoundedLeewardStorybook,
	HighLeewardStorybook,
}

/// Typed authored geometry for one leeward varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeewardItem {
	TemperateConifer(&'static LeewardTemperateConifer),
	Storybook(&'static LeewardStorybook),
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct LeewardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SHELTERED_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(10.0, 18.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDBREAK_TEMPERATE_CONIFER: LeewardTemperateConifer = LeewardTemperateConifer {
	height: UnitRange::new(16.0, 24.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ROUNDED_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(10.0, 18.0),
	stalk_radius: UnitRange::new(0.16, 0.34),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const HIGH_LEEWARD_STORYBOOK: LeewardStorybook = LeewardStorybook {
	height: UnitRange::new(16.0, 24.0),
	stalk_radius: UnitRange::new(0.18, 0.40),
	canopy_spread: UnitRange::new(3.0, 7.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHELTERED_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const SHELTERED_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

const WINDBREAK_TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "temperate_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDBREAK_TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "blue_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const LEEWARD_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const LEEWARD_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl LeewardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.65`; the `None` weight of `6.8` puts the placed share at
	/// `2.65 / 9.45 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let sheltered_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let windbreak_temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.66));
		let rounded_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let high_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(4.0),
			GroveBucket::placed(1.8, sheltered_temperate, Self::ShelteredTemperateConifer),
			GroveBucket::placed(1.6, windbreak_temperate, Self::WindbreakTemperateConifer),
			GroveBucket::placed(2.4, rounded_storybook, Self::RoundedLeewardStorybook),
			GroveBucket::placed(0.45, high_storybook, Self::HighLeewardStorybook),
		])
	}

	pub fn item(self) -> LeewardItem {
		match self {
			Self::ShelteredTemperateConifer => {
				LeewardItem::TemperateConifer(&SHELTERED_TEMPERATE_CONIFER)
			}
			Self::WindbreakTemperateConifer => {
				LeewardItem::TemperateConifer(&WINDBREAK_TEMPERATE_CONIFER)
			}
			Self::RoundedLeewardStorybook => LeewardItem::Storybook(&ROUNDED_LEEWARD_STORYBOOK),
			Self::HighLeewardStorybook => LeewardItem::Storybook(&HIGH_LEEWARD_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_STICK_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_STICK_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_STICK_MIX
			}
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShelteredTemperateConifer => SHELTERED_TEMPERATE_CANOPY_MIX,
			Self::WindbreakTemperateConifer => WINDBREAK_TEMPERATE_CANOPY_MIX,
			Self::RoundedLeewardStorybook | Self::HighLeewardStorybook => {
				LEEWARD_STORYBOOK_CANOPY_MIX
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = LeewardCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 4.0);
		assert_eq!(dist.buckets[1].item, Some(LeewardCell::ShelteredTemperateConifer));
		assert_eq!(dist.buckets[1].weight, 1.8);
		assert_eq!(dist.buckets[2].item, Some(LeewardCell::WindbreakTemperateConifer));
		assert_eq!(dist.buckets[2].weight, 1.6);
		assert_eq!(dist.buckets[3].item, Some(LeewardCell::RoundedLeewardStorybook));
		assert_eq!(dist.buckets[3].weight, 2.4);
		assert_eq!(dist.buckets[4].item, Some(LeewardCell::HighLeewardStorybook));
		assert_eq!(dist.buckets[4].weight, 0.45);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = LeewardCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.61).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let LeewardItem::TemperateConifer(sheltered) =
			LeewardCell::ShelteredTemperateConifer.item()
		else {
			anyhow::bail!("expected sheltered temperate conifer item");
		};
		assert_eq!(sheltered.height, UnitRange::new(10.0, 18.0));
		assert_eq!(sheltered.canopy_density, MODERATE_CANOPY_DENSITY);

		let LeewardItem::TemperateConifer(windbreak) =
			LeewardCell::WindbreakTemperateConifer.item()
		else {
			anyhow::bail!("expected windbreak temperate conifer item");
		};
		assert_eq!(windbreak.height, UnitRange::new(16.0, 24.0));
		assert_eq!(windbreak.canopy_density, SPARSE_CANOPY_DENSITY);

		let LeewardItem::Storybook(rounded) = LeewardCell::RoundedLeewardStorybook.item() else {
			anyhow::bail!("expected rounded leeward storybook item");
		};
		assert_eq!(rounded.height, UnitRange::new(10.0, 18.0));
		assert_eq!(rounded.canopy_density, DENSE_CANOPY_DENSITY);

		let LeewardItem::Storybook(high) = LeewardCell::HighLeewardStorybook.item() else {
			anyhow::bail!("expected high leeward storybook item");
		};
		assert_eq!(high.height, UnitRange::new(16.0, 24.0));
		assert_eq!(high.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = LeewardCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let sheltered = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(LeewardCell::ShelteredTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing sheltered temperate bucket"))?;
		assert_eq!(sheltered.constraints.steepness.end, 0.50);

		let windbreak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(LeewardCell::WindbreakTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing windbreak temperate bucket"))?;
		assert_eq!(windbreak.constraints.steepness.end, 0.66);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_sheltered_conifer_but_falls_through_to_windbreak() -> Result<()> {
		let prepared =
			LeewardCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.45 };
		let sheltered_outcome = prepared.select_from(1, Vec3::new(5.0, 0.40, 5.0), 1.0, &moderate);
		match sheltered_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LeewardCell::ShelteredTemperateConifer);
			}
			other => {
				anyhow::bail!("expected ShelteredTemperateConifer on moderate slope, got {other:?}")
			}
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
		let steep_outcome = prepared.select_from(1, Vec3::new(5.0, 0.40, 5.0), 1.0, &steep);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LeewardCell::WindbreakTemperateConifer);
			}
			other => {
				anyhow::bail!("expected fall-through to WindbreakTemperateConifer, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			LeewardCell::ShelteredTemperateConifer,
			LeewardCell::WindbreakTemperateConifer,
			LeewardCell::RoundedLeewardStorybook,
			LeewardCell::HighLeewardStorybook,
		] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
