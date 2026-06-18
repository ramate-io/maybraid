//! Jerry's Chaparral — well-known moderately dense dry scrub understory grove
//! ([RFC-183 §3.4.5.7], [#318](https://github.com/ramate-io/maybraid/issues/318)).
//!
//! Mixes Rory's Head-trained forms, Common High Bush chaparral mass, and rare small Friend's
//! Conifer accents. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{JerrysChaparral, JerrysChaparralStd};

/// RFC `projection_count: Moderate` — chaparral high-bush varietal.
const MODERATE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.32, 0.48);
const MODERATE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.58, 0.78);

/// Authored Jerry's Chaparral grove definition.
///
/// Cell footprint sits at the RFC midpoint (`6.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JerrysChaparralCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(6.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-6.5, 6.5)),
		distribution: JerrysChaparralCell::distribution(),
	}
}

/// Ordered chaparral varietals ([RFC-183 §3.4.5.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JerrysChaparralCell {
	DryRoryHeadTrained,
	ChaparralHighBush,
	SmallFriendsConifer,
	ManzanitaRory,
}

/// Typed authored geometry for one chaparral varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JerrysChaparralItem {
	RoryHead(&'static JerrysChaparralRoryHead),
	Bush(&'static JerrysChaparralBush),
	FriendsConifer(&'static JerrysChaparralFriendsConifer),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralRoryHead {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.030 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one small Friend's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct JerrysChaparralFriendsConifer {
	pub height: UnitRange,
	/// World-space stalk base radius (RFC `0.025 × H`).
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const DRY_RORY_HEAD: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.20, 3.20),
	stalk_radius: UnitRange::new(0.036, 0.096),
	canopy_spread: UnitRange::new(0.80, 2.00),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const MANZANITA_RORY: JerrysChaparralRoryHead = JerrysChaparralRoryHead {
	height: UnitRange::new(1.40, 3.00),
	stalk_radius: UnitRange::new(0.042, 0.090),
	canopy_spread: UnitRange::new(0.90, 2.10),
	canopy_density: UnitRange::new(0.0, 0.35),
};

const CHAPARRAL_HIGH_BUSH: JerrysChaparralBush = JerrysChaparralBush {
	height: UnitRange::new(1.00, 2.40),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: MODERATE_PROJECTION_RADIAL,
	vertical_bias: MODERATE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const SMALL_FRIENDS_CONIFER: JerrysChaparralFriendsConifer = JerrysChaparralFriendsConifer {
	height: UnitRange::new(2.00, 6.00),
	stalk_radius: UnitRange::new(0.05, 0.15),
	canopy_spread: UnitRange::new(0.50, 1.40),
	canopy_density: UnitRange::new(0.35, 0.65),
};

const DRY_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "gray_brown"),
	PaletteSlot::new("vine_bark", "tan_brown"),
]);

const DRY_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("scrub_green", "pale_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

const CHAPARRAL_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_bark", "tan_brown"),
	PaletteSlot::new("gray_brown", "straw_brown"),
]);

const CHAPARRAL_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_green", "olive_green"),
	PaletteSlot::new("scrub_green", "tan_green"),
	PaletteSlot::new("dark_green", "pale_green"),
]);

const FRIENDS_CONIFER_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("conifer_bark", "dark_bark"),
	PaletteSlot::new("gray_brown", "dry_bark"),
]);

const FRIENDS_CONIFER_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "blue_green"),
	PaletteSlot::new("dry_green", "deep_green"),
	PaletteSlot::new("olive_green", "needle_green"),
]);

const MANZANITA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("manzanita_red", "copper_red"),
	PaletteSlot::new("smooth_burgundy", "orange_bark"),
]);

const MANZANITA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "pale_green"),
	PaletteSlot::new("flower_white", "dry_green"),
	PaletteSlot::new("dark_green", "yellow_green"),
]);

impl JerrysChaparralCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.3` (RFC relative proportions); the `None` weight of `7.0` puts
	/// the placed share at `4.3 / 11.3 ≈ 0.38`, mid RFC `DENSITY_RANGE` (`0.24..0.52`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_rory =
			PlacementConstraints::new(UnitRange::new(0.10, 0.65), UnitRange::new(0.0, 0.78));
		let bush = PlacementConstraints::new(UnitRange::new(0.05, 0.70), UnitRange::new(0.0, 0.55));
		let conifer =
			PlacementConstraints::new(UnitRange::new(0.15, 0.75), UnitRange::new(0.0, 0.65));
		let manzanita =
			PlacementConstraints::new(UnitRange::new(0.15, 0.70), UnitRange::new(0.0, 0.72));
		GroveDistribution::new(vec![
			GroveBucket::none(7.0),
			GroveBucket::placed(1.5, dry_rory, Self::DryRoryHeadTrained),
			GroveBucket::placed(2.0, bush, Self::ChaparralHighBush),
			GroveBucket::placed(0.45, conifer, Self::SmallFriendsConifer),
			GroveBucket::placed(0.35, manzanita, Self::ManzanitaRory),
		])
	}

	pub fn item(self) -> JerrysChaparralItem {
		match self {
			Self::DryRoryHeadTrained => JerrysChaparralItem::RoryHead(&DRY_RORY_HEAD),
			Self::ChaparralHighBush => JerrysChaparralItem::Bush(&CHAPARRAL_HIGH_BUSH),
			Self::SmallFriendsConifer => {
				JerrysChaparralItem::FriendsConifer(&SMALL_FRIENDS_CONIFER)
			}
			Self::ManzanitaRory => JerrysChaparralItem::RoryHead(&MANZANITA_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_STICK_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_STICK_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_STICK_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DryRoryHeadTrained => DRY_RORY_CANOPY_MIX,
			Self::ChaparralHighBush => CHAPARRAL_BUSH_CANOPY_MIX,
			Self::SmallFriendsConifer => FRIENDS_CONIFER_CANOPY_MIX,
			Self::ManzanitaRory => MANZANITA_RORY_CANOPY_MIX,
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
		let dist = JerrysChaparralCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 7.0);
		assert_eq!(dist.buckets[1].item, Some(JerrysChaparralCell::DryRoryHeadTrained));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(JerrysChaparralCell::ChaparralHighBush));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(JerrysChaparralCell::SmallFriendsConifer));
		assert_eq!(dist.buckets[3].weight, 0.45);
		assert_eq!(dist.buckets[4].item, Some(JerrysChaparralCell::ManzanitaRory));
		assert_eq!(dist.buckets[4].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = JerrysChaparralCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.24..=0.52).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn rory_bush_and_conifer_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			JerrysChaparralCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						("rory", JerrysChaparralItem::RoryHead(_)) => true,
						("bush", JerrysChaparralItem::Bush(_)) => true,
						("conifer", JerrysChaparralItem::FriendsConifer(_)) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let rory = weight("rory");
		let bush = weight("bush");
		let conifer = weight("conifer");
		assert!((rory - 1.85).abs() < 1e-4, "expected rory weight 1.85, got {rory}");
		assert!((bush - 2.0).abs() < 1e-4, "expected bush weight 2.0, got {bush}");
		assert!((conifer - 0.45).abs() < 1e-4, "expected conifer weight 0.45, got {conifer}");
		Ok(())
	}

	#[test]
	fn rory_bush_and_conifer_geometry_follows_authored_bands() -> Result<()> {
		let JerrysChaparralItem::RoryHead(dry) = JerrysChaparralCell::DryRoryHeadTrained.item()
		else {
			anyhow::bail!("expected dry rory item");
		};
		assert!(dry.height.start >= 1.20);
		assert!(dry.height.end <= 3.20);

		let JerrysChaparralItem::Bush(bush) = JerrysChaparralCell::ChaparralHighBush.item() else {
			anyhow::bail!("expected bush item");
		};
		assert_eq!(bush.shoot_count, 7..=11);
		assert!(bush.leaf_radius.end <= 0.11);

		let JerrysChaparralItem::FriendsConifer(conifer) =
			JerrysChaparralCell::SmallFriendsConifer.item()
		else {
			anyhow::bail!("expected conifer item");
		};
		assert!(conifer.height.end <= 6.00);

		let JerrysChaparralItem::RoryHead(manzanita) = JerrysChaparralCell::ManzanitaRory.item()
		else {
			anyhow::bail!("expected manzanita rory item");
		};
		assert!(manzanita.canopy_density.end <= 0.35);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// ChaparralHighBush (index 2) rejects steepness 0.60; first-fit falls to SmallFriendsConifer
		// (index 3), which allows steepness up to 0.65.
		let prepared = JerrysChaparralCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.60 };
		let outcome = prepared.select_from(2, Vec3::new(5.0, 0.35, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, JerrysChaparralCell::SmallFriendsConifer);
			}
			other => anyhow::bail!("expected SmallFriendsConifer fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
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
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
