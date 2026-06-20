//! Trade Winds — low-density tropical upper-canopy grove
//! ([RFC-183 §3.4.7.15], [#337](https://github.com/ramate-io/maybraid/issues/337)).
//!
//! Common Storybook forms with less common Sope and Honu banyans and rare Waialea palms. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{TradeWinds, TradeWindsStd};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);
/// Sparse sampled descender-density band ([`0.02`, `0.04`]).
const SPARSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.02, 0.04);

/// Authored Trade Winds grove definition.
///
/// Cell footprint sits at the RFC midpoint (`26.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TradeWindsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(26.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-26.0, 26.0),
		),
		distribution: TradeWindsCell::distribution(),
	}
}

/// Ordered trade-winds varietals ([RFC-183 §3.4.7.15]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeWindsCell {
	TradeStorybook,
	TradeSopesBanyan,
	TradeHonuBanyan,
	RareTallTradeStorybook,
	RareTradeWaialeaPalm,
}

/// Typed authored geometry for one trade-winds varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeWindsItem {
	Storybook(&'static TradeWindsStorybook),
	Sope(&'static TradeWindsBanyan),
	Honu(&'static TradeWindsBanyan),
	WaialeaPalm(&'static TradeWindsWaialeaPalm),
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeWindsStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Sope or Honu banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeWindsBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeWindsWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const TRADE_STORYBOOK: TradeWindsStorybook = TradeWindsStorybook {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.16, 0.36),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TRADE_SOPE_BANYAN: TradeWindsBanyan = TradeWindsBanyan {
	height: UnitRange::new(10.0, 25.0),
	stalk_radius: UnitRange::new(0.22, 0.52),
	canopy_spread: UnitRange::new(4.0, 10.0),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const TRADE_HONU_BANYAN: TradeWindsBanyan = TradeWindsBanyan {
	height: UnitRange::new(10.0, 25.0),
	stalk_radius: UnitRange::new(0.22, 0.52),
	canopy_spread: UnitRange::new(4.0, 10.0),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_TALL_TRADE_STORYBOOK: TradeWindsStorybook = TradeWindsStorybook {
	height: UnitRange::new(20.0, 30.0),
	stalk_radius: UnitRange::new(0.18, 0.42),
	canopy_spread: UnitRange::new(3.0, 8.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const RARE_TRADE_WAIALEA_PALM: TradeWindsWaialeaPalm = TradeWindsWaialeaPalm {
	height: UnitRange::new(10.0, 40.0),
	crown_density: MODERATE_CANOPY_DENSITY,
};

const TRADE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const TRADE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "bright_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "wet_brown"),
	PaletteSlot::new("green_brown", "dark_bark"),
]);

const SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
]);

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const RARE_TALL_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const RARE_TALL_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

impl TradeWindsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.1`; the `None` weight of `21.5` puts the placed share at
	/// `4.1 / 25.6 ≈ 0.16`, mid RFC `DENSITY_RANGE` (`0.08..0.24`).
	pub fn distribution() -> GroveDistribution<Self> {
		let trade_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.48));
		let trade_sope =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.44));
		let trade_honu =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.42));
		let rare_tall_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		let rare_waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(21.5),
			GroveBucket::placed(2.0, trade_storybook, Self::TradeStorybook),
			GroveBucket::placed(0.75, trade_sope, Self::TradeSopesBanyan),
			GroveBucket::placed(0.75, trade_honu, Self::TradeHonuBanyan),
			GroveBucket::placed(0.35, rare_tall_storybook, Self::RareTallTradeStorybook),
			GroveBucket::placed(0.25, rare_waialea, Self::RareTradeWaialeaPalm),
		])
	}

	pub fn item(self) -> TradeWindsItem {
		match self {
			Self::TradeStorybook => TradeWindsItem::Storybook(&TRADE_STORYBOOK),
			Self::TradeSopesBanyan => TradeWindsItem::Sope(&TRADE_SOPE_BANYAN),
			Self::TradeHonuBanyan => TradeWindsItem::Honu(&TRADE_HONU_BANYAN),
			Self::RareTallTradeStorybook => TradeWindsItem::Storybook(&RARE_TALL_TRADE_STORYBOOK),
			Self::RareTradeWaialeaPalm => TradeWindsItem::WaialeaPalm(&RARE_TRADE_WAIALEA_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TradeStorybook => TRADE_STORYBOOK_STICK_MIX,
			Self::TradeSopesBanyan => SOPE_STICK_MIX,
			Self::TradeHonuBanyan => HONU_STICK_MIX,
			Self::RareTallTradeStorybook => RARE_TALL_STORYBOOK_STICK_MIX,
			Self::RareTradeWaialeaPalm => WAIALEA_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TradeStorybook => TRADE_STORYBOOK_CANOPY_MIX,
			Self::TradeSopesBanyan => SOPE_CANOPY_MIX,
			Self::TradeHonuBanyan => HONU_CANOPY_MIX,
			Self::RareTallTradeStorybook => RARE_TALL_STORYBOOK_CANOPY_MIX,
			Self::RareTradeWaialeaPalm => WAIALEA_CANOPY_MIX,
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
	use gimme_gen::Cell;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = TradeWindsCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 21.5);
		assert_eq!(dist.buckets[1].item, Some(TradeWindsCell::TradeStorybook));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(TradeWindsCell::TradeSopesBanyan));
		assert_eq!(dist.buckets[2].weight, 0.75);
		assert_eq!(dist.buckets[3].item, Some(TradeWindsCell::TradeHonuBanyan));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(TradeWindsCell::RareTallTradeStorybook));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(TradeWindsCell::RareTradeWaialeaPalm));
		assert_eq!(dist.buckets[5].weight, 0.25);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TradeWindsCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let TradeWindsItem::Storybook(trade) = TradeWindsCell::TradeStorybook.item() else {
			anyhow::bail!("expected trade storybook item");
		};
		assert_eq!(trade.height, UnitRange::new(10.0, 20.0));
		assert_eq!(trade.canopy_density, MODERATE_CANOPY_DENSITY);

		let TradeWindsItem::Honu(honu) = TradeWindsCell::TradeHonuBanyan.item() else {
			anyhow::bail!("expected trade honu item");
		};
		assert_eq!(honu.height, UnitRange::new(10.0, 25.0));
		assert_eq!(honu.descender_density, SPARSE_DESCENDER_DENSITY);

		let TradeWindsItem::WaialeaPalm(palm) = TradeWindsCell::RareTradeWaialeaPalm.item() else {
			anyhow::bail!("expected rare waialea item");
		};
		assert_eq!(palm.height, UnitRange::new(10.0, 40.0));
		assert_eq!(palm.crown_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = TradeWindsCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let sope = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TradeWindsCell::TradeSopesBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing trade sope bucket"))?;
		assert_eq!(sope.constraints.steepness.end, 0.44);

		let tall = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(TradeWindsCell::RareTallTradeStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing rare tall storybook bucket"))?;
		assert_eq!(tall.constraints.steepness.end, 0.50);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_trade_sope_but_allows_rare_tall_storybook() -> Result<()> {
		let prepared =
			TradeWindsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.40 };
		let sope_outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &moderate);
		match sope_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, TradeWindsCell::TradeSopesBanyan);
			}
			other => anyhow::bail!("expected TradeSopesBanyan on moderate slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.45 };
		let steep_outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &steep);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, TradeWindsCell::RareTallTradeStorybook);
			}
			other => {
				anyhow::bail!("expected RareTallTradeStorybook on steep slope, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TradeWindsCell::TradeStorybook,
			TradeWindsCell::TradeSopesBanyan,
			TradeWindsCell::TradeHonuBanyan,
			TradeWindsCell::RareTallTradeStorybook,
			TradeWindsCell::RareTradeWaialeaPalm,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
