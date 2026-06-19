//! Conifer Massives — low-density giant evergreen upper-canopy grove
//! ([RFC-183 §3.4.7.2], [#343](https://github.com/ramate-io/maybraid/issues/343)).
//!
//! Towering Northern, Friend's, Liam's, and Temperate Conifer skyline forms above conifer lower
//! massives. Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{ConiferMassives, ConiferMassivesStd};

/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Conifer Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`50.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ConiferMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(50.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-50.0, 50.0),
		),
		distribution: ConiferMassivesCell::distribution(),
	}
}

/// Ordered conifer-massive varietals ([RFC-183 §3.4.7.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConiferMassivesCell {
	MassiveNorthernConifer,
	MassiveFriendsConifer,
	MassiveLiamsConifer,
	MassiveTemperateConifer,
}

/// Typed authored geometry for one conifer-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConiferMassivesItem {
	NorthernConifer(&'static ConiferMassivesNorthernConifer),
	FriendsConifer(&'static ConiferMassivesFriendsConifer),
	LiamsConifer(&'static ConiferMassivesLiamsConifer),
	TemperateConifer(&'static ConiferMassivesTemperateConifer),
}

/// Authored geometry ranges for one Northern Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesNorthernConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesFriendsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Temperate Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct ConiferMassivesTemperateConifer {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

const MASSIVE_NORTHERN_CONIFER: ConiferMassivesNorthernConifer = ConiferMassivesNorthernConifer {
	height: UnitRange::new(70.0, 200.0),
	stalk_radius: UnitRange::new(2.0, 6.5),
	canopy_spread: UnitRange::new(15.0, 45.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_FRIENDS_CONIFER: ConiferMassivesFriendsConifer = ConiferMassivesFriendsConifer {
	height: UnitRange::new(100.0, 130.0),
	stalk_radius: UnitRange::new(2.5, 5.5),
	canopy_spread: UnitRange::new(18.0, 35.0),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_LIAMS_CONIFER: ConiferMassivesLiamsConifer = ConiferMassivesLiamsConifer {
	height: UnitRange::new(25.0, 130.0),
	stalk_radius: UnitRange::new(0.5, 4.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const MASSIVE_TEMPERATE_CONIFER: ConiferMassivesTemperateConifer =
	ConiferMassivesTemperateConifer {
		height: UnitRange::new(40.0, 120.0),
		canopy_density: MODERATE_CANOPY_DENSITY,
	};

const NORTHERN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "conifer_bark"),
]);

const NORTHERN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("cold_green", "blue_green"),
	PaletteSlot::new("deep_green", "dark_green"),
]);

const FRIENDS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const FRIENDS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "blue_green"),
	PaletteSlot::new("dark_green", "fresh_green"),
]);

const TEMPERATE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("temperate_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "moss_bark"),
]);

const TEMPERATE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("soft_green", "deep_green"),
	PaletteSlot::new("blue_green", "fresh_green"),
]);

impl ConiferMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.5`; the `None` weight of `23.0` puts the placed share at
	/// `3.5 / 26.5 ≈ 0.132`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let northern =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.70));
		let friends =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let liams =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.76));
		let temperate =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		GroveDistribution::new(vec![
			GroveBucket::none(23.0),
			GroveBucket::placed(1.25, northern, Self::MassiveNorthernConifer),
			GroveBucket::placed(1.25, friends, Self::MassiveFriendsConifer),
			GroveBucket::placed(0.75, liams, Self::MassiveLiamsConifer),
			GroveBucket::placed(0.25, temperate, Self::MassiveTemperateConifer),
		])
	}

	pub fn item(self) -> ConiferMassivesItem {
		match self {
			Self::MassiveNorthernConifer => {
				ConiferMassivesItem::NorthernConifer(&MASSIVE_NORTHERN_CONIFER)
			}
			Self::MassiveFriendsConifer => {
				ConiferMassivesItem::FriendsConifer(&MASSIVE_FRIENDS_CONIFER)
			}
			Self::MassiveLiamsConifer => ConiferMassivesItem::LiamsConifer(&MASSIVE_LIAMS_CONIFER),
			Self::MassiveTemperateConifer => {
				ConiferMassivesItem::TemperateConifer(&MASSIVE_TEMPERATE_CONIFER)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveNorthernConifer => NORTHERN_STICK_MIX,
			Self::MassiveFriendsConifer => FRIENDS_STICK_MIX,
			Self::MassiveLiamsConifer => LIAMS_STICK_MIX,
			Self::MassiveTemperateConifer => TEMPERATE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveNorthernConifer => NORTHERN_CANOPY_MIX,
			Self::MassiveFriendsConifer => FRIENDS_CANOPY_MIX,
			Self::MassiveLiamsConifer => LIAMS_CANOPY_MIX,
			Self::MassiveTemperateConifer => TEMPERATE_CANOPY_MIX,
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
		let dist = ConiferMassivesCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 23.0);
		assert_eq!(dist.buckets[1].item, Some(ConiferMassivesCell::MassiveNorthernConifer));
		assert_eq!(dist.buckets[1].weight, 1.25);
		assert_eq!(dist.buckets[2].item, Some(ConiferMassivesCell::MassiveFriendsConifer));
		assert_eq!(dist.buckets[2].weight, 1.25);
		assert_eq!(dist.buckets[3].item, Some(ConiferMassivesCell::MassiveLiamsConifer));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(ConiferMassivesCell::MassiveTemperateConifer));
		assert_eq!(dist.buckets[4].weight, 0.25);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ConiferMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ConiferMassivesItem::NorthernConifer(northern) =
			ConiferMassivesCell::MassiveNorthernConifer.item()
		else {
			anyhow::bail!("expected northern conifer item");
		};
		assert_eq!(northern.height, UnitRange::new(70.0, 200.0));
		assert_eq!(northern.canopy_density, DENSE_CANOPY_DENSITY);

		let ConiferMassivesItem::FriendsConifer(friends) =
			ConiferMassivesCell::MassiveFriendsConifer.item()
		else {
			anyhow::bail!("expected friends conifer item");
		};
		assert_eq!(friends.height, UnitRange::new(100.0, 130.0));

		let ConiferMassivesItem::LiamsConifer(liams) =
			ConiferMassivesCell::MassiveLiamsConifer.item()
		else {
			anyhow::bail!("expected liams conifer item");
		};
		assert_eq!(liams.height, UnitRange::new(25.0, 130.0));
		assert_eq!(liams.canopy_density, MODERATE_CANOPY_DENSITY);

		let ConiferMassivesItem::TemperateConifer(temperate) =
			ConiferMassivesCell::MassiveTemperateConifer.item()
		else {
			anyhow::bail!("expected temperate conifer item");
		};
		assert_eq!(temperate.height, UnitRange::new(40.0, 120.0));
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = ConiferMassivesCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let northern = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ConiferMassivesCell::MassiveNorthernConifer))
			.ok_or_else(|| anyhow::anyhow!("missing northern bucket"))?;
		assert_eq!(northern.constraints.steepness.end, 0.70);

		let friends = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ConiferMassivesCell::MassiveFriendsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing friends bucket"))?;
		assert_eq!(friends.constraints.steepness.end, 0.64);

		let temperate = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ConiferMassivesCell::MassiveTemperateConifer))
			.ok_or_else(|| anyhow::anyhow!("missing temperate bucket"))?;
		assert_eq!(temperate.constraints.steepness.end, 0.58);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_friends_but_allows_liams() -> Result<()> {
		let prepared = ConiferMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.68 };
		let outcome = prepared.select_from(3, Vec3::new(5.0, 0.40, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ConiferMassivesCell::MassiveFriendsConifer);
				assert_ne!(variant, ConiferMassivesCell::MassiveNorthernConifer);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ConiferMassivesCell::MassiveNorthernConifer,
			ConiferMassivesCell::MassiveFriendsConifer,
			ConiferMassivesCell::MassiveLiamsConifer,
			ConiferMassivesCell::MassiveTemperateConifer,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
