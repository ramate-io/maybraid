//! Date Grove — moderate-density cultivated Date Palm upper-canopy grove
//! ([RFC-183 §3.4.7.9], [#357](https://github.com/ramate-io/maybraid/issues/357)).
//!
//! Single moderate-crown date palm form with tight cell offset on warm flat terrain.
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
pub use render::{DateGrove, DateGroveStd};

/// Moderate sampled crown-density band ([`0.35`, `0.65`]).
const MODERATE_CROWN_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Date Grove definition.
///
/// Cell footprint sits at the RFC midpoint (`12.0` m). Offset stays tight so placements read as
/// cultivated rows.
pub fn definition() -> GroveDefinition<DateGroveCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(12.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-2.0, 2.0),
		),
		distribution: DateGroveCell::distribution(),
	}
}

/// Ordered date-grove varietals ([RFC-183 §3.4.7.9]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateGroveCell {
	FruitingDatePalm,
}

/// Typed authored geometry for one date-grove varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateGroveItem {
	DatePalm(&'static DateGroveDatePalm),
}

/// Authored geometry ranges for one Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct DateGroveDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

const FRUITING_DATE_PALM: DateGroveDatePalm = DateGroveDatePalm {
	height: UnitRange::new(5.0, 8.0),
	crown_density: MODERATE_CROWN_DENSITY,
};

const DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("date_trunk", "dry_brown"),
]);

const DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_green", "olive_green"),
	PaletteSlot::new("fresh_green", "yellow_green"),
]);

impl DateGroveCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weight `1.0`; the `None` weight of `2.4` puts the placed share at
	/// `1.0 / 3.4 ≈ 0.29`, mid RFC `DENSITY_RANGE` (`0.22..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting_date =
			PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.30));
		GroveDistribution::new(vec![
			GroveBucket::none(2.4),
			GroveBucket::placed(1.0, fruiting_date, Self::FruitingDatePalm),
		])
	}

	pub fn item(self) -> DateGroveItem {
		match self {
			Self::FruitingDatePalm => DateGroveItem::DatePalm(&FRUITING_DATE_PALM),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		DATE_PALM_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		DATE_PALM_CANOPY_MIX
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
	use anyhow::Result;
	use bevy_math::Vec3;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = DateGroveCell::distribution();
		assert_eq!(dist.len(), 2);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 2.4);
		assert_eq!(dist.buckets[1].item, Some(DateGroveCell::FruitingDatePalm));
		assert_eq!(dist.buckets[1].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = DateGroveCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.22..=0.42).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let DateGroveItem::DatePalm(palm) = DateGroveCell::FruitingDatePalm.item() else {
			anyhow::bail!("expected fruiting date palm item");
		};
		assert_eq!(palm.height, UnitRange::new(5.0, 8.0));
		assert_eq!(palm.crown_density, MODERATE_CROWN_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = DateGroveCell::distribution();
		let palm = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DateGroveCell::FruitingDatePalm))
			.ok_or_else(|| anyhow::anyhow!("missing fruiting date palm bucket"))?;
		assert_eq!(palm.constraints.elevation.start, 0.0);
		assert_eq!(palm.constraints.elevation.end, 0.46);
		assert_eq!(palm.constraints.steepness.end, 0.30);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [DateGroveCell::FruitingDatePalm] {
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0));
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.10 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
