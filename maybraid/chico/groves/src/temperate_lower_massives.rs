//! Temperate Lower Massives — massive lower-canopy grove beneath very tall upper canopy
//! ([RFC-183 §3.4.6.9], [#330](https://github.com/ramate-io/maybraid/issues/330)).
//!
//! Common 10–20 m braid oak and storybook forms with rare Rory's Head-trained accents.
//! Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{TemperateLowerMassives, TemperateLowerMassivesStd};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);

/// Authored Temperate Lower Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`26` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TemperateLowerMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(18.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-26.0, 26.0),
		),
		distribution: TemperateLowerMassivesCell::distribution(),
	}
}

/// Ordered temperate lower-massive varietals ([RFC-183 §3.4.6.9]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperateLowerMassivesCell {
	LowerMassiveBraidOak,
	LowerMassiveStorybook,
	RareLowerMassiveRory,
}

/// Typed authored geometry for one temperate lower-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperateLowerMassivesItem {
	BraidOak(&'static TemperateLowerMassivesBraidOak),
	Storybook(&'static TemperateLowerMassivesStorybook),
	Rory(&'static TemperateLowerMassivesRory),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one rare Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateLowerMassivesRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const LOWER_MASSIVE_BRAID_OAK: TemperateLowerMassivesBraidOak = TemperateLowerMassivesBraidOak {
	height: UnitRange::new(8.0, 24.0),
	canopy_spread: UnitRange::new(3.0, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_STORYBOOK: TemperateLowerMassivesStorybook = TemperateLowerMassivesStorybook {
	height: UnitRange::new(8.0, 20.0),
	stalk_radius: UnitRange::new(0.36, 0.72),
	canopy_spread: UnitRange::new(3.5, 8.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_LOWER_MASSIVE_RORY: TemperateLowerMassivesRory = TemperateLowerMassivesRory {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.12, 0.30),
	canopy_spread: UnitRange::new(2.5, 6.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("oak_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("dark_green", "light_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

impl TemperateLowerMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.35` (RFC relative proportions); the `None` weight of `19.0` puts
	/// the placed share at `4.35 / 23.35 ≈ 0.19`, mid RFC `DENSITY_RANGE` (`0.10..0.26`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.68), UnitRange::new(0.0, 0.50));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.00, 0.72), UnitRange::new(0.0, 0.56));
		let rory = PlacementConstraints::new(UnitRange::new(0.00, 0.64), UnitRange::new(0.0, 0.68));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, braid_oak, Self::LowerMassiveBraidOak),
			GroveBucket::placed(2.0, storybook, Self::LowerMassiveStorybook),
			GroveBucket::placed(0.35, rory, Self::RareLowerMassiveRory),
		])
	}

	pub fn item(self) -> TemperateLowerMassivesItem {
		match self {
			Self::LowerMassiveBraidOak => {
				TemperateLowerMassivesItem::BraidOak(&LOWER_MASSIVE_BRAID_OAK)
			}
			Self::LowerMassiveStorybook => {
				TemperateLowerMassivesItem::Storybook(&LOWER_MASSIVE_STORYBOOK)
			}
			Self::RareLowerMassiveRory => {
				TemperateLowerMassivesItem::Rory(&RARE_LOWER_MASSIVE_RORY)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveBraidOak => BRAID_OAK_STICK_MIX,
			Self::LowerMassiveStorybook => STORYBOOK_STICK_MIX,
			Self::RareLowerMassiveRory => RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveBraidOak => BRAID_OAK_CANOPY_MIX,
			Self::LowerMassiveStorybook => STORYBOOK_CANOPY_MIX,
			Self::RareLowerMassiveRory => RORY_CANOPY_MIX,
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
		let dist = TemperateLowerMassivesCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 8.0);
		assert_eq!(dist.buckets[1].item, Some(TemperateLowerMassivesCell::LowerMassiveBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(TemperateLowerMassivesCell::LowerMassiveStorybook));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(TemperateLowerMassivesCell::RareLowerMassiveRory));
		assert_eq!(dist.buckets[3].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TemperateLowerMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.10..=0.36).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let TemperateLowerMassivesItem::BraidOak(oak) =
			TemperateLowerMassivesCell::LowerMassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(8.0, 24.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let TemperateLowerMassivesItem::Storybook(story) =
			TemperateLowerMassivesCell::LowerMassiveStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(8.0, 20.0));
		assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);

		let TemperateLowerMassivesItem::Rory(rory) =
			TemperateLowerMassivesCell::RareLowerMassiveRory.item()
		else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(10.0, 20.0));
		assert_eq!(rory.canopy_spread, UnitRange::new(2.5, 6.0));
		assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = TemperateLowerMassivesCell::distribution();
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateLowerMassivesCell::LowerMassiveBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.elevation.start, 0.00);
		assert_eq!(braid_oak.constraints.elevation.end, 0.68);
		assert_eq!(braid_oak.constraints.steepness.end, 0.50);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateLowerMassivesCell::LowerMassiveStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.00);
		assert_eq!(storybook.constraints.elevation.end, 0.72);
		assert_eq!(storybook.constraints.steepness.end, 0.56);

		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateLowerMassivesCell::RareLowerMassiveRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.elevation.start, 0.00);
		assert_eq!(rory.constraints.elevation.end, 0.64);
		assert_eq!(rory.constraints.steepness.end, 0.68);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_braid_oak_but_allows_rory() -> Result<()> {
		let prepared = TemperateLowerMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
		let outcome = prepared.select_from(8, Vec3::new(5.0, 0.30, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, TemperateLowerMassivesCell::LowerMassiveBraidOak);
				assert_ne!(variant, TemperateLowerMassivesCell::LowerMassiveStorybook);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TemperateLowerMassivesCell::LowerMassiveBraidOak,
			TemperateLowerMassivesCell::LowerMassiveStorybook,
			TemperateLowerMassivesCell::RareLowerMassiveRory,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
