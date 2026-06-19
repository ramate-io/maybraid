//! Temperate Massives — low-density giant broadleaf upper-canopy grove
//! ([RFC-183 §3.4.7.3], [#345](https://github.com/ramate-io/maybraid/issues/345)).
//!
//! Enormous Braid Oak, Storybook Tree, and rare Rory's Head-trained skyline forms above temperate
//! lower massives. Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{TemperateMassives, TemperateMassivesStd};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Temperate Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`49` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TemperateMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(49.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-49.0, 49.0),
		),
		distribution: TemperateMassivesCell::distribution(),
	}
}

/// Ordered temperate-massive varietals ([RFC-183 §3.4.7.3]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperateMassivesCell {
	MassiveBraidOak,
	MassiveStorybook,
	RareMassiveRory,
}

/// Typed authored geometry for one temperate-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperateMassivesItem {
	BraidOak(&'static TemperateMassivesBraidOak),
	Storybook(&'static TemperateMassivesStorybook),
	Rory(&'static TemperateMassivesRory),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one rare Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemperateMassivesRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const MASSIVE_BRAID_OAK: TemperateMassivesBraidOak = TemperateMassivesBraidOak {
	height: UnitRange::new(28.0, 80.0),
	canopy_spread: UnitRange::new(8.0, 20.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_STORYBOOK: TemperateMassivesStorybook = TemperateMassivesStorybook {
	height: UnitRange::new(35.0, 170.0),
	stalk_radius: UnitRange::new(3.0, 9.0),
	canopy_spread: UnitRange::new(12.0, 35.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RARE_MASSIVE_RORY: TemperateMassivesRory = TemperateMassivesRory {
	height: UnitRange::new(50.0, 200.0),
	stalk_radius: UnitRange::new(0.45, 1.80),
	canopy_spread: UnitRange::new(6.0, 14.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
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

impl TemperateMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.35`; the `None` weight of `24.6` puts the placed share at
	/// `4.35 / 28.95 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.08..0.22`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(24.6),
			GroveBucket::placed(2.0, braid_oak, Self::MassiveBraidOak),
			GroveBucket::placed(2.0, storybook, Self::MassiveStorybook),
			GroveBucket::placed(0.35, rory, Self::RareMassiveRory),
		])
	}

	pub fn item(self) -> TemperateMassivesItem {
		match self {
			Self::MassiveBraidOak => TemperateMassivesItem::BraidOak(&MASSIVE_BRAID_OAK),
			Self::MassiveStorybook => TemperateMassivesItem::Storybook(&MASSIVE_STORYBOOK),
			Self::RareMassiveRory => TemperateMassivesItem::Rory(&RARE_MASSIVE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_STICK_MIX,
			Self::MassiveStorybook => STORYBOOK_STICK_MIX,
			Self::RareMassiveRory => RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveBraidOak => BRAID_OAK_CANOPY_MIX,
			Self::MassiveStorybook => STORYBOOK_CANOPY_MIX,
			Self::RareMassiveRory => RORY_CANOPY_MIX,
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
		let dist = TemperateMassivesCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.6);
		assert_eq!(dist.buckets[1].item, Some(TemperateMassivesCell::MassiveBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(TemperateMassivesCell::MassiveStorybook));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(TemperateMassivesCell::RareMassiveRory));
		assert_eq!(dist.buckets[3].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TemperateMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.22).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let TemperateMassivesItem::BraidOak(oak) = TemperateMassivesCell::MassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(28.0, 80.0));
		assert_eq!(oak.canopy_spread, UnitRange::new(8.0, 20.0));
		assert_eq!(oak.canopy_density, DENSE_CANOPY_DENSITY);

		let TemperateMassivesItem::Storybook(story) =
			TemperateMassivesCell::MassiveStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(35.0, 170.0));
		assert_eq!(story.stalk_radius, UnitRange::new(3.0, 9.0));
		assert_eq!(story.canopy_spread, UnitRange::new(12.0, 35.0));
		assert_eq!(story.canopy_density, DENSE_CANOPY_DENSITY);

		let TemperateMassivesItem::Rory(rory) = TemperateMassivesCell::RareMassiveRory.item() else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(50.0, 200.0));
		assert_eq!(rory.canopy_spread, UnitRange::new(6.0, 14.0));
		assert_eq!(rory.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = TemperateMassivesCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::MassiveBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.steepness.end, 0.44);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::MassiveStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.steepness.end, 0.50);

		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TemperateMassivesCell::RareMassiveRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.steepness.end, 0.60);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_braid_oak_but_allows_rory() -> Result<()> {
		let prepared =
			TemperateMassivesCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
		let outcome = prepared.select_from(8, Vec3::new(5.0, 0.30, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, TemperateMassivesCell::MassiveBraidOak);
				assert_ne!(variant, TemperateMassivesCell::MassiveStorybook);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TemperateMassivesCell::MassiveBraidOak,
			TemperateMassivesCell::MassiveStorybook,
			TemperateMassivesCell::RareMassiveRory,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(400.0, 1.0, 400.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
