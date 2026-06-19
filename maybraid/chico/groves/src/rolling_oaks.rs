//! Rolling Oaks — low-density open oak-country upper-canopy grove
//! ([RFC-183 §3.4.7.5], [#349](https://github.com/ramate-io/maybraid/issues/349)).
//!
//! Common dry Braid Oak forms with rare Storybook accents across rolling open woodland. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{RollingOaks, RollingOaksStd};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Rolling Oaks grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RollingOaksCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: RollingOaksCell::distribution(),
	}
}

/// Ordered rolling-oaks varietals ([RFC-183 §3.4.7.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingOaksCell {
	RollingBraidOak,
	RareRollingStorybook,
}

/// Typed authored geometry for one rolling-oaks varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RollingOaksItem {
	BraidOak(&'static RollingOaksBraidOak),
	Storybook(&'static RollingOaksStorybook),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingOaksStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const ROLLING_BRAID_OAK: RollingOaksBraidOak = RollingOaksBraidOak {
	height: UnitRange::new(5.0, 20.0),
	canopy_spread: UnitRange::new(2.0, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_ROLLING_STORYBOOK: RollingOaksStorybook = RollingOaksStorybook {
	height: UnitRange::new(5.0, 20.0),
	stalk_radius: UnitRange::new(0.12, 0.32),
	canopy_spread: UnitRange::new(2.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ROLLING_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const ROLLING_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "dry_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const ROLLING_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

impl RollingOaksCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.35`; the `None` weight of `12.4` puts the placed share at
	/// `2.35 / 14.75 ≈ 0.16`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.08, 0.72), UnitRange::new(0.0, 0.48));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.08, 0.68), UnitRange::new(0.0, 0.54));
		GroveDistribution::new(vec![
			GroveBucket::none(12.4),
			GroveBucket::placed(2.0, braid_oak, Self::RollingBraidOak),
			GroveBucket::placed(0.35, storybook, Self::RareRollingStorybook),
		])
	}

	pub fn item(self) -> RollingOaksItem {
		match self {
			Self::RollingBraidOak => RollingOaksItem::BraidOak(&ROLLING_BRAID_OAK),
			Self::RareRollingStorybook => RollingOaksItem::Storybook(&RARE_ROLLING_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_STICK_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::RollingBraidOak => ROLLING_BRAID_OAK_CANOPY_MIX,
			Self::RareRollingStorybook => ROLLING_STORYBOOK_CANOPY_MIX,
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
		let dist = RollingOaksCell::distribution();
		assert_eq!(dist.len(), 3);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 12.4);
		assert_eq!(dist.buckets[1].item, Some(RollingOaksCell::RollingBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(RollingOaksCell::RareRollingStorybook));
		assert_eq!(dist.buckets[2].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = RollingOaksCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let RollingOaksItem::BraidOak(oak) = RollingOaksCell::RollingBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(5.0, 20.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let RollingOaksItem::Storybook(story) = RollingOaksCell::RareRollingStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(5.0, 20.0));
		assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = RollingOaksCell::distribution();
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RollingBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.elevation.start, 0.08);
		assert_eq!(braid_oak.constraints.elevation.end, 0.72);
		assert_eq!(braid_oak.constraints.steepness.end, 0.48);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RollingOaksCell::RareRollingStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.08);
		assert_eq!(storybook.constraints.elevation.end, 0.68);
		assert_eq!(storybook.constraints.steepness.end, 0.54);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_braid_oak_but_allows_storybook() -> Result<()> {
		let prepared =
			RollingOaksCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let story_outcome = prepared.select_from(5, Vec3::new(5.0, 0.40, 5.0), 1.0, &terrain);
		match story_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RollingOaksCell::RareRollingStorybook);
			}
			other => anyhow::bail!("expected RareRollingStorybook on moderate slope, got {other:?}"),
		}
		let braid_outcome = prepared.select_from(1, Vec3::new(5.0, 0.40, 5.0), 1.0, &terrain);
		match braid_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, RollingOaksCell::RollingBraidOak);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [RollingOaksCell::RollingBraidOak, RollingOaksCell::RareRollingStorybook] {
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
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
