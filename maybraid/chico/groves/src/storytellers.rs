//! Storyteller's — colorful whimsical Storybook and Braid Oak upper-canopy grove
//! ([RFC-183 §3.4.7.14], [#336](https://github.com/ramate-io/maybraid/issues/336)).
//!
//! Moderate-density color-pop canopy with common storybook and braid-oak forms. Forest-layer
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
pub use render::{Storytellers, StorytellersStd};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Storyteller's grove definition.
///
/// Cell footprint sits at the RFC midpoint (`22.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<StorytellersCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(22.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-22.0, 22.0),
		),
		distribution: StorytellersCell::distribution(),
	}
}

/// Ordered storyteller varietals ([RFC-183 §3.4.7.14]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorytellersCell {
	ColorfulStorybook,
	ColorfulBraidOak,
	BrightCanopyStorybook,
	PinkLanternStorybook,
	RedFestivalBraidOak,
	PurpleCrownStorybook,
	BlueMoonStorybook,
}

/// Typed authored geometry for one storyteller varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorytellersItem {
	Storybook(&'static StorytellersStorybook),
	BraidOak(&'static StorytellersBraidOak),
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct StorytellersBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

const COLORFUL_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 30.0),
	stalk_radius: UnitRange::new(0.18, 0.42),
	canopy_spread: UnitRange::new(3.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const COLORFUL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(10.0, 30.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const BRIGHT_CANOPY_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(10.0, 26.0),
	stalk_radius: UnitRange::new(0.16, 0.38),
	canopy_spread: UnitRange::new(2.5, 7.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PINK_LANTERN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(8.0, 18.0),
	stalk_radius: UnitRange::new(0.14, 0.32),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RED_FESTIVAL_BRAID_OAK: StorytellersBraidOak = StorytellersBraidOak {
	height: UnitRange::new(12.0, 24.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PURPLE_CROWN_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(14.0, 30.0),
	stalk_radius: UnitRange::new(0.20, 0.45),
	canopy_spread: UnitRange::new(3.5, 9.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const BLUE_MOON_STORYBOOK: StorytellersStorybook = StorytellersStorybook {
	height: UnitRange::new(12.0, 22.0),
	stalk_radius: UnitRange::new(0.16, 0.36),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLORFUL_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const COLORFUL_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("storybook_green", "gold_green"),
	PaletteSlot::new("rose_leaf", "fresh_green"),
]);

const COLORFUL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "copper_red"),
	PaletteSlot::new("oak_bark", "dark_bark"),
]);

const COLORFUL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "gold_green"),
	PaletteSlot::new("copper_leaf", "fresh_green"),
]);

const BRIGHT_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "red_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BRIGHT_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gold_leaf", "fresh_green"),
	PaletteSlot::new("rose_leaf", "light_green"),
]);

const PINK_LANTERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("warm_bark", "purple_brown"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const PINK_LANTERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("hot_pink", "rose_leaf"),
	PaletteSlot::new("fresh_green", "pink_bloom"),
]);

const RED_FESTIVAL_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_oak_bark", "bright_red_bark"),
	PaletteSlot::new("copper_red", "dark_bark"),
]);

const RED_FESTIVAL_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_leaf", "copper_leaf"),
	PaletteSlot::new("gold_leaf", "fresh_green"),
]);

const PURPLE_CROWN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("purple_brown", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const PURPLE_CROWN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("violet_leaf", "purple_leaf"),
	PaletteSlot::new("deep_green", "rose_leaf"),
]);

const BLUE_MOON_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_gray_bark", "purple_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const BLUE_MOON_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_leaf", "cyan_leaf"),
	PaletteSlot::new("deep_blue_green", "fresh_green"),
]);

impl StorytellersCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.9`; the `None` weight of `12.6` puts the placed share at
	/// `4.9 / 17.5 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let colorful_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.52));
		let colorful_braid =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let bright_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.56));
		let pink_lantern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let red_festival =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.46));
		let purple_crown =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.54));
		let blue_moon =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(12.6),
			GroveBucket::placed(1.5, colorful_storybook, Self::ColorfulStorybook),
			GroveBucket::placed(1.5, colorful_braid, Self::ColorfulBraidOak),
			GroveBucket::placed(0.75, bright_storybook, Self::BrightCanopyStorybook),
			GroveBucket::placed(0.35, pink_lantern, Self::PinkLanternStorybook),
			GroveBucket::placed(0.30, red_festival, Self::RedFestivalBraidOak),
			GroveBucket::placed(0.25, purple_crown, Self::PurpleCrownStorybook),
			GroveBucket::placed(0.25, blue_moon, Self::BlueMoonStorybook),
		])
	}

	pub fn item(self) -> StorytellersItem {
		match self {
			Self::ColorfulStorybook => StorytellersItem::Storybook(&COLORFUL_STORYBOOK),
			Self::ColorfulBraidOak => StorytellersItem::BraidOak(&COLORFUL_BRAID_OAK),
			Self::BrightCanopyStorybook => StorytellersItem::Storybook(&BRIGHT_CANOPY_STORYBOOK),
			Self::PinkLanternStorybook => StorytellersItem::Storybook(&PINK_LANTERN_STORYBOOK),
			Self::RedFestivalBraidOak => StorytellersItem::BraidOak(&RED_FESTIVAL_BRAID_OAK),
			Self::PurpleCrownStorybook => StorytellersItem::Storybook(&PURPLE_CROWN_STORYBOOK),
			Self::BlueMoonStorybook => StorytellersItem::Storybook(&BLUE_MOON_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_STICK_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_STICK_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_STICK_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_STICK_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_STICK_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_STICK_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ColorfulStorybook => COLORFUL_STORYBOOK_CANOPY_MIX,
			Self::ColorfulBraidOak => COLORFUL_BRAID_OAK_CANOPY_MIX,
			Self::BrightCanopyStorybook => BRIGHT_STORYBOOK_CANOPY_MIX,
			Self::PinkLanternStorybook => PINK_LANTERN_CANOPY_MIX,
			Self::RedFestivalBraidOak => RED_FESTIVAL_CANOPY_MIX,
			Self::PurpleCrownStorybook => PURPLE_CROWN_CANOPY_MIX,
			Self::BlueMoonStorybook => BLUE_MOON_CANOPY_MIX,
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
		let dist = StorytellersCell::distribution();
		assert_eq!(dist.len(), 8);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 12.6);
		assert_eq!(dist.buckets[1].item, Some(StorytellersCell::ColorfulStorybook));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(StorytellersCell::ColorfulBraidOak));
		assert_eq!(dist.buckets[2].weight, 1.5);
		assert_eq!(dist.buckets[3].item, Some(StorytellersCell::BrightCanopyStorybook));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(StorytellersCell::PinkLanternStorybook));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(StorytellersCell::RedFestivalBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.30);
		assert_eq!(dist.buckets[6].item, Some(StorytellersCell::PurpleCrownStorybook));
		assert_eq!(dist.buckets[6].weight, 0.25);
		assert_eq!(dist.buckets[7].item, Some(StorytellersCell::BlueMoonStorybook));
		assert_eq!(dist.buckets[7].weight, 0.25);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = StorytellersCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let StorytellersItem::Storybook(colorful) = StorytellersCell::ColorfulStorybook.item()
		else {
			anyhow::bail!("expected colorful storybook item");
		};
		assert_eq!(colorful.height, UnitRange::new(10.0, 30.0));
		assert_eq!(colorful.canopy_density, DENSE_CANOPY_DENSITY);

		let StorytellersItem::BraidOak(festival) = StorytellersCell::RedFestivalBraidOak.item()
		else {
			anyhow::bail!("expected red festival braid oak item");
		};
		assert_eq!(festival.height, UnitRange::new(12.0, 24.0));
		assert_eq!(festival.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = StorytellersCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let braid = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(StorytellersCell::ColorfulBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing colorful braid oak bucket"))?;
		assert_eq!(braid.constraints.steepness.end, 0.48);

		let bright = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(StorytellersCell::BrightCanopyStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing bright storybook bucket"))?;
		assert_eq!(bright.constraints.steepness.end, 0.56);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_colorful_braid_oak_but_allows_bright_storybook() -> Result<()> {
		let prepared =
			StorytellersCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.40 };
		let braid_outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, &moderate);
		match braid_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, StorytellersCell::ColorfulBraidOak);
			}
			other => anyhow::bail!("expected ColorfulBraidOak on moderate slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let steep_outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, &steep);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, StorytellersCell::BrightCanopyStorybook);
			}
			other => {
				anyhow::bail!("expected BrightCanopyStorybook on steep slope, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			StorytellersCell::ColorfulStorybook,
			StorytellersCell::ColorfulBraidOak,
			StorytellersCell::BrightCanopyStorybook,
			StorytellersCell::PinkLanternStorybook,
			StorytellersCell::RedFestivalBraidOak,
			StorytellersCell::PurpleCrownStorybook,
			StorytellersCell::BlueMoonStorybook,
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
