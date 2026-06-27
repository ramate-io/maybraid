//! Conifer Sapling — well-known moderate-density young conifer lower-canopy grove
//! ([RFC-183 §3.4.6.5], [#326](https://github.com/ramate-io/maybraid/issues/326)).
//!
//! Mixed Friend's and Northern Conifer saplings beneath taller evergreen canopy. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Standard sapling height band ([`1.0`, `4.0`] m).
const SAPLING_HEIGHT: UnitRange = UnitRange::new(1.0, 4.0);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate band for windswept northern accents.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.20, 0.55);

/// Authored Conifer Sapling grove definition.
///
/// Cell footprint at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid.
pub fn definition() -> GroveDefinition<ConiferSaplingCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(10.5),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: ConiferSaplingCell::distribution(),
	}
}

/// Ordered conifer-sapling varietals ([RFC-183 §3.4.6.5]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConiferSaplingCell {
	FriendSapling,
	NorthernSapling,
	MossyFriendSapling,
	ColdNorthernSapling,
	BrightFriendSapling,
	WindsweptNorthernSapling,
}

/// Typed authored geometry for one conifer-sapling varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConiferSaplingItem {
	FriendsConifer(&'static ConiferSaplingFriendsConifer),
	NorthernConifer(&'static ConiferSaplingNorthernConifer),
}

/// Authored geometry ranges for one Friend's Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Northern Conifer sapling form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferSaplingNorthernConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (Northern `0.032 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const MOSSY_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.15, 0.55),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const BRIGHT_FRIEND_SAPLING: ConiferSaplingFriendsConifer = ConiferSaplingFriendsConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.025, 0.10),
	canopy_spread: UnitRange::new(0.22, 0.75),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.20, 0.70),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const COLD_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.18, 0.60),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WINDSWEPT_NORTHERN_SAPLING: ConiferSaplingNorthernConifer = ConiferSaplingNorthernConifer {
	height: SAPLING_HEIGHT,
	stalk_radius: UnitRange::new(0.032, 0.128),
	canopy_spread: UnitRange::new(0.12, 0.50),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const MOSSY_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const MOSSY_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const BRIGHT_FRIEND_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("young_bark", "conifer_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const BRIGHT_FRIEND_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "yellow_green"),
	PaletteSlot::new("light_green", "spring_green"),
]);

const NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const COLD_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "gray_brown"),
	PaletteSlot::new("conifer_bark", "dark_bark"),
]);

const COLD_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "deep_green"),
	PaletteSlot::new("blue_green", "dark_green"),
]);

const WINDSWEPT_NORTHERN_SAPLING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gray_brown", "cold_bark"),
	PaletteSlot::new("conifer_bark", "dry_bark"),
]);

const WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("blue_green", "cold_green"),
	PaletteSlot::new("needle_green", "olive_green"),
]);

impl ConiferSaplingCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.4` (RFC pair plus sapling accents); the `None` weight of `5.2` puts
	/// the placed share at `3.4 / 8.6 ≈ 0.40`, mid RFC `DENSITY_RANGE` (`0.28..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let friend =
			PlacementConstraints::new(UnitRange::new(0.18, 0.82), UnitRange::new(0.0, 0.64));
		let northern =
			PlacementConstraints::new(UnitRange::new(0.22, 0.88), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(5.2),
			GroveBucket::placed(1.0, friend, Self::FriendSapling),
			GroveBucket::placed(1.0, northern, Self::NorthernSapling),
			GroveBucket::placed(0.35, friend, Self::MossyFriendSapling),
			GroveBucket::placed(0.35, northern, Self::ColdNorthernSapling),
			GroveBucket::placed(0.30, friend, Self::BrightFriendSapling),
			GroveBucket::placed(0.40, northern, Self::WindsweptNorthernSapling),
		])
	}

	pub fn item(self) -> ConiferSaplingItem {
		match self {
			Self::FriendSapling => ConiferSaplingItem::FriendsConifer(&FRIEND_SAPLING),
			Self::BrightFriendSapling => ConiferSaplingItem::FriendsConifer(&BRIGHT_FRIEND_SAPLING),
			Self::MossyFriendSapling => ConiferSaplingItem::FriendsConifer(&MOSSY_FRIEND_SAPLING),
			Self::NorthernSapling => ConiferSaplingItem::NorthernConifer(&NORTHERN_SAPLING),
			Self::ColdNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&COLD_NORTHERN_SAPLING)
			}
			Self::WindsweptNorthernSapling => {
				ConiferSaplingItem::NorthernConifer(&WINDSWEPT_NORTHERN_SAPLING)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_STICK_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_STICK_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_STICK_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_STICK_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_STICK_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FriendSapling => FRIEND_SAPLING_CANOPY_MIX,
			Self::MossyFriendSapling => MOSSY_FRIEND_SAPLING_CANOPY_MIX,
			Self::BrightFriendSapling => BRIGHT_FRIEND_SAPLING_CANOPY_MIX,
			Self::NorthernSapling => NORTHERN_SAPLING_CANOPY_MIX,
			Self::ColdNorthernSapling => COLD_NORTHERN_SAPLING_CANOPY_MIX,
			Self::WindsweptNorthernSapling => WINDSWEPT_NORTHERN_SAPLING_CANOPY_MIX,
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
		let dist = ConiferSaplingCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 5.2);
		assert_eq!(dist.buckets[1].item, Some(ConiferSaplingCell::FriendSapling));
		assert_eq!(dist.buckets[1].weight, 1.0);
		assert_eq!(dist.buckets[2].item, Some(ConiferSaplingCell::NorthernSapling));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(ConiferSaplingCell::MossyFriendSapling));
		assert_eq!(dist.buckets[3].weight, 0.35);
		assert_eq!(dist.buckets[4].item, Some(ConiferSaplingCell::ColdNorthernSapling));
		assert_eq!(dist.buckets[4].weight, 0.35);
		assert_eq!(dist.buckets[5].item, Some(ConiferSaplingCell::BrightFriendSapling));
		assert_eq!(dist.buckets[5].weight, 0.30);
		assert_eq!(dist.buckets[6].item, Some(ConiferSaplingCell::WindsweptNorthernSapling));
		assert_eq!(dist.buckets[6].weight, 0.40);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ConiferSaplingCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.28..=0.48).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ConiferSaplingItem::FriendsConifer(friend) = ConiferSaplingCell::FriendSapling.item()
		else {
			anyhow::bail!("expected friend sapling item");
		};
		assert_eq!(friend.height, SAPLING_HEIGHT);
		assert_eq!(friend.canopy_density, MODERATE_CANOPY_DENSITY);

		let ConiferSaplingItem::NorthernConifer(northern) =
			ConiferSaplingCell::NorthernSapling.item()
		else {
			anyhow::bail!("expected northern sapling item");
		};
		assert_eq!(northern.height, SAPLING_HEIGHT);

		let ConiferSaplingItem::NorthernConifer(windswept) =
			ConiferSaplingCell::WindsweptNorthernSapling.item()
		else {
			anyhow::bail!("expected windswept northern item");
		};
		assert_eq!(windswept.canopy_density, SPARSE_TO_MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_selects_per_bucket() -> Result<()> {
		let prepared = ConiferSaplingCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);

		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.50, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::FriendSapling);
			}
			other => anyhow::bail!("expected FriendSapling at mid elevation, got {other:?}"),
		}

		// Friend max elevation is 0.82; Northern accepts up to 0.88.
		let high_terrain = FlatTerrainSample { elevation: 0.85, steepness: 0.30 };
		let outcome = prepared.select_from(
			2,
			Vec3::new(6.0, 0.85, 6.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&high_terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::NorthernSapling);
			}
			other => anyhow::bail!("expected NorthernSapling at high elevation, got {other:?}"),
		}

		// Friend max steepness is 0.64; Northern accepts up to 0.72.
		let steep_terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.70 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(7.0, 0.50, 7.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep_terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ConiferSaplingCell::NorthernSapling);
			}
			other => anyhow::bail!("expected NorthernSapling on steep slope, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
		let placements = grove.populate(&extent, &terrain);
		assert!(!placements.is_empty());

		let cell = definition().cell_extent_xz.x;
		let off_center = placements
			.iter()
			.filter(|p| {
				let local_x = (p.position.x / cell).fract() - 0.5;
				let local_z = (p.position.z / cell).fract() - 0.5;
				local_x.abs() > 0.25 || local_z.abs() > 0.25
			})
			.count();
		assert!(
			off_center * 2 >= placements.len(),
			"expected at least half of {} placements off cell centers, got {off_center}",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let grove = Grove::assemble(
			definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
