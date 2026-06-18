//! Jungle Lower Massives — massive lower-canopy grove beneath very tall upper canopy
//! ([RFC-183 §3.4.6.7], [#328](https://github.com/ramate-io/maybraid/issues/328)).
//!
//! Common 10–20 m jungle storybook and banyan forms with rare braid-oak accents. Forest-layer
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
pub use render::{JungleLowerMassives, JungleLowerMassivesStd};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Jungle Lower Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`23` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JungleLowerMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(18.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-23.0, 23.0),
		),
		distribution: JungleLowerMassivesCell::distribution(),
	}
}

/// Ordered jungle lower-massive varietals ([RFC-183 §3.4.6.7]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleLowerMassivesCell {
	LowerMassiveJungleStorybook,
	LowerMassiveHonuBanyan,
	LowerMassiveSopesBanyan,
	LowerMassiveWaialeaPalm,
	RareLowerMassiveBraidOak,
}

/// Typed authored geometry for one jungle lower-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JungleLowerMassivesItem {
	JungleStorybook(&'static JungleLowerMassivesJungleStorybook),
	Honu(&'static JungleLowerMassivesBanyan),
	Sope(&'static JungleLowerMassivesBanyan),
	WaialeaPalm(&'static JungleLowerMassivesWaialeaPalm),
	BraidOak(&'static JungleLowerMassivesBraidOak),
}

/// Authored geometry ranges for one Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one rare Braid Oak accent.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const LOWER_MASSIVE_JUNGLE_STORYBOOK: JungleLowerMassivesJungleStorybook =
	JungleLowerMassivesJungleStorybook {
		height: UnitRange::new(10.0, 20.0),
		canopy_density: DENSE_CANOPY_DENSITY,
		jungle_growth_density: MODERATE_CANOPY_DENSITY,
	};

const LOWER_MASSIVE_HONU_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_SOPE_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_WAIALEA_PALM: JungleLowerMassivesWaialeaPalm = JungleLowerMassivesWaialeaPalm {
	height: UnitRange::new(10.0, 20.0),
	crown_density: DENSE_CANOPY_DENSITY,
};

const RARE_LOWER_MASSIVE_BRAID_OAK: JungleLowerMassivesBraidOak = JungleLowerMassivesBraidOak {
	height: UnitRange::new(10.0, 20.0),
	canopy_spread: UnitRange::new(3.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const JUNGLE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_jungle_bark", "wet_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const JUNGLE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
]);

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "green_brown"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "yellow_green"),
]);

impl JungleLowerMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `6.35` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `6.35 / 17.35 ≈ 0.37`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.54), UnitRange::new(0.0, 0.54));
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.46));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.48), UnitRange::new(0.0, 0.50));
		let waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.62));
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.02, 0.50), UnitRange::new(0.0, 0.52));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, jungle_storybook, Self::LowerMassiveJungleStorybook),
			GroveBucket::placed(2.0, honu, Self::LowerMassiveHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::LowerMassiveSopesBanyan),
			GroveBucket::placed(1.0, waialea, Self::LowerMassiveWaialeaPalm),
			GroveBucket::placed(0.35, braid_oak, Self::RareLowerMassiveBraidOak),
		])
	}

	pub fn item(self) -> JungleLowerMassivesItem {
		match self {
			Self::LowerMassiveJungleStorybook => {
				JungleLowerMassivesItem::JungleStorybook(&LOWER_MASSIVE_JUNGLE_STORYBOOK)
			}
			Self::LowerMassiveHonuBanyan => {
				JungleLowerMassivesItem::Honu(&LOWER_MASSIVE_HONU_BANYAN)
			}
			Self::LowerMassiveSopesBanyan => {
				JungleLowerMassivesItem::Sope(&LOWER_MASSIVE_SOPE_BANYAN)
			}
			Self::LowerMassiveWaialeaPalm => {
				JungleLowerMassivesItem::WaialeaPalm(&LOWER_MASSIVE_WAIALEA_PALM)
			}
			Self::RareLowerMassiveBraidOak => {
				JungleLowerMassivesItem::BraidOak(&RARE_LOWER_MASSIVE_BRAID_OAK)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::LowerMassiveHonuBanyan => HONU_STICK_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_STICK_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_STICK_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::LowerMassiveHonuBanyan => HONU_CANOPY_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_CANOPY_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_CANOPY_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_CANOPY_MIX,
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
		let dist = JungleLowerMassivesCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 8.0);
		assert_eq!(
			dist.buckets[1].item,
			Some(JungleLowerMassivesCell::LowerMassiveJungleStorybook)
		);
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(JungleLowerMassivesCell::LowerMassiveSopesBanyan));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm));
		assert_eq!(dist.buckets[4].weight, 1.0);
		assert_eq!(dist.buckets[5].item, Some(JungleLowerMassivesCell::RareLowerMassiveBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = JungleLowerMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.45).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let JungleLowerMassivesItem::JungleStorybook(jungle) =
			JungleLowerMassivesCell::LowerMassiveJungleStorybook.item()
		else {
			anyhow::bail!("expected jungle storybook item");
		};
		assert_eq!(jungle.height, UnitRange::new(10.0, 20.0));

		let JungleLowerMassivesItem::Honu(honu) =
			JungleLowerMassivesCell::LowerMassiveHonuBanyan.item()
		else {
			anyhow::bail!("expected honu item");
		};
		assert_eq!(honu.height, UnitRange::new(10.0, 20.0));
		assert_eq!(honu.canopy_density, DENSE_CANOPY_DENSITY);

		let JungleLowerMassivesItem::BraidOak(oak) =
			JungleLowerMassivesCell::RareLowerMassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn waialea_accepts_steeper_slope_than_honu() -> Result<()> {
		let dist = JungleLowerMassivesCell::distribution();
		let honu = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing honu bucket"))?;
		let waialea = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm))
			.ok_or_else(|| anyhow::anyhow!("missing waialea bucket"))?;
		assert!(waialea.constraints.steepness.end > honu.constraints.steepness.end);
		assert_eq!(honu.constraints.steepness.end, 0.46);
		assert_eq!(waialea.constraints.steepness.end, 0.62);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_honu_but_allows_waialea() -> Result<()> {
		let prepared = JungleLowerMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
		let outcome = prepared.select_from(8, Vec3::new(5.0, 0.30, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, JungleLowerMassivesCell::LowerMassiveHonuBanyan);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			JungleLowerMassivesCell::LowerMassiveJungleStorybook,
			JungleLowerMassivesCell::LowerMassiveHonuBanyan,
			JungleLowerMassivesCell::LowerMassiveSopesBanyan,
			JungleLowerMassivesCell::LowerMassiveWaialeaPalm,
			JungleLowerMassivesCell::RareLowerMassiveBraidOak,
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
