//! Alpine — cold upland conifer upper-canopy grove
//! ([RFC-183 §3.4.7.12], [#334](https://github.com/ramate-io/maybraid/issues/334)).
//!
//! Tall Friend's Conifer with less common Liam's Conifer on high, steep terrain. Forest-layer
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
pub use render::{Alpine, AlpineStd};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Alpine grove definition.
///
/// Cell footprint sits at the RFC midpoint (`27.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<AlpineCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(27.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-27.0, 27.0),
		),
		distribution: AlpineCell::distribution(),
	}
}

/// Ordered alpine varietals ([RFC-183 §3.4.7.12]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpineCell {
	TallAlpineFriendsConifer,
	WindlineFriendsConifer,
	AlpineLiamsConifer,
	NeedleSpireLiamsConifer,
}

/// Typed authored geometry for one alpine varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlpineItem {
	FriendsConifer(&'static AlpineFriendsConifer),
	LiamsConifer(&'static AlpineLiamsConifer),
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct AlpineLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

const TALL_ALPINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(18.0, 40.0),
	stalk_radius: UnitRange::new(0.32, 0.72),
	canopy_spread: UnitRange::new(4.0, 12.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const WINDLINE_FRIENDS: AlpineFriendsConifer = AlpineFriendsConifer {
	height: UnitRange::new(10.0, 22.0),
	stalk_radius: UnitRange::new(0.18, 0.42),
	canopy_spread: UnitRange::new(1.5, 5.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ALPINE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(10.0, 40.0),
	stalk_radius: UnitRange::new(0.25, 0.85),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NEEDLE_SPIRE_LIAMS: AlpineLiamsConifer = AlpineLiamsConifer {
	height: UnitRange::new(16.0, 32.0),
	stalk_radius: UnitRange::new(0.30, 0.55),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const TALL_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const TALL_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const WINDLINE_FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wind_barked", "cold_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const WINDLINE_FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("dark_green", "deep_green"),
]);

const ALPINE_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const ALPINE_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const NEEDLE_SPIRE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("stone_gray", "conifer_bark"),
]);

const NEEDLE_SPIRE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "dark_green"),
	PaletteSlot::new("cold_green", "deep_green"),
]);

impl AlpineCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.7`; the `None` weight of `9.5` puts the placed share at
	/// `3.7 / 13.2 ≈ 0.28`, mid RFC `DENSITY_RANGE` (`0.18..0.38`).
	pub fn distribution() -> GroveDistribution<Self> {
		let tall_friends = PlacementConstraints::new(
			UnitRange::new(0.42, 1.0),
			UnitRange::new(0.0, 0.68),
		);
		let windline_friends = PlacementConstraints::new(
			UnitRange::new(0.62, 1.0),
			UnitRange::new(0.0, 0.86),
		);
		let alpine_liams = PlacementConstraints::new(
			UnitRange::new(0.50, 1.0),
			UnitRange::new(0.0, 0.86),
		);
		let needle_spire = PlacementConstraints::new(
			UnitRange::new(0.58, 1.0),
			UnitRange::new(0.0, 0.92),
		);
		GroveDistribution::new(vec![
			GroveBucket::none(9.5),
			GroveBucket::placed(1.5, tall_friends, Self::TallAlpineFriendsConifer),
			GroveBucket::placed(0.75, windline_friends, Self::WindlineFriendsConifer),
			GroveBucket::placed(1.0, alpine_liams, Self::AlpineLiamsConifer),
			GroveBucket::placed(0.45, needle_spire, Self::NeedleSpireLiamsConifer),
		])
	}

	pub fn item(self) -> AlpineItem {
		match self {
			Self::TallAlpineFriendsConifer | Self::WindlineFriendsConifer => match self {
				Self::TallAlpineFriendsConifer => {
					AlpineItem::FriendsConifer(&TALL_ALPINE_FRIENDS)
				}
				Self::WindlineFriendsConifer => AlpineItem::FriendsConifer(&WINDLINE_FRIENDS),
				_ => unreachable!(),
			},
			Self::AlpineLiamsConifer => AlpineItem::LiamsConifer(&ALPINE_LIAMS),
			Self::NeedleSpireLiamsConifer => AlpineItem::LiamsConifer(&NEEDLE_SPIRE_LIAMS),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_STICK_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_STICK_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_STICK_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::TallAlpineFriendsConifer => TALL_FRIENDS_CANOPY_MIX,
			Self::WindlineFriendsConifer => WINDLINE_FRIENDS_CANOPY_MIX,
			Self::AlpineLiamsConifer => ALPINE_LIAMS_CANOPY_MIX,
			Self::NeedleSpireLiamsConifer => NEEDLE_SPIRE_CANOPY_MIX,
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
		let dist = AlpineCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 9.5);
		assert_eq!(dist.buckets[1].item, Some(AlpineCell::TallAlpineFriendsConifer));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(AlpineCell::WindlineFriendsConifer));
		assert_eq!(dist.buckets[2].weight, 0.75);
		assert_eq!(dist.buckets[3].item, Some(AlpineCell::AlpineLiamsConifer));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(AlpineCell::NeedleSpireLiamsConifer));
		assert_eq!(dist.buckets[4].weight, 0.45);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = AlpineCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let AlpineItem::FriendsConifer(tall) = AlpineCell::TallAlpineFriendsConifer.item() else {
			anyhow::bail!("expected tall friends item");
		};
		assert_eq!(tall.height, UnitRange::new(18.0, 40.0));
		assert_eq!(tall.canopy_density, DENSE_CANOPY_DENSITY);

		let AlpineItem::LiamsConifer(spire) = AlpineCell::NeedleSpireLiamsConifer.item() else {
			anyhow::bail!("expected needle spire item");
		};
		assert_eq!(spire.height, UnitRange::new(16.0, 32.0));
		assert_eq!(spire.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = AlpineCell::distribution();
		let tall = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(AlpineCell::TallAlpineFriendsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing tall friends bucket"))?;
		assert_eq!(tall.constraints.elevation.start, 0.42);
		assert_eq!(tall.constraints.steepness.end, 0.68);

		let windline = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(AlpineCell::WindlineFriendsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing windline friends bucket"))?;
		assert_eq!(windline.constraints.elevation.start, 0.62);
		assert_eq!(windline.constraints.steepness.end, 0.86);
		Ok(())
	}

	#[test]
	fn low_elevation_rejects_alpine_but_high_steep_ridge_allows_windline() -> Result<()> {
		let prepared =
			AlpineCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let low = FlatTerrainSample { elevation: 0.30, steepness: 0.20 };
		let low_outcome = prepared.select_from(1, Vec3::new(5.0, 0.30, 5.0), 1.0, &low);
		match low_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, AlpineCell::TallAlpineFriendsConifer);
				assert_ne!(variant, AlpineCell::AlpineLiamsConifer);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		let ridge = FlatTerrainSample { elevation: 0.72, steepness: 0.78 };
		let windline_outcome =
			prepared.select_from(2, Vec3::new(5.0, 0.72, 5.0), 1.0, &ridge);
		match windline_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, AlpineCell::WindlineFriendsConifer);
			}
			other => anyhow::bail!("expected WindlineFriendsConifer on high steep ridge, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			AlpineCell::TallAlpineFriendsConifer,
			AlpineCell::WindlineFriendsConifer,
			AlpineCell::AlpineLiamsConifer,
			AlpineCell::NeedleSpireLiamsConifer,
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
		let terrain = FlatTerrainSample { elevation: 0.65, steepness: 0.35 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
