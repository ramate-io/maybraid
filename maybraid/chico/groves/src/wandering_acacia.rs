//! Wandering Acacia — very-low-density dry open upper-canopy grove
//! ([RFC-183 §3.4.7.16], [#338](https://github.com/ramate-io/maybraid/issues/338)).
//!
//! Sparse acacia-like High Bush, dry Sope's Banyan, and rare vase and torch accents across open
//! country. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.02, 0.65);
/// Sparse sampled descender-density band ([`0.02`, `0.04`]).
const SPARSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.01, 0.04);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Wandering Acacia grove definition.
///
/// Cell footprint sits at the RFC midpoint (`37.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<WanderingAcaciaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(37.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-37.0, 37.0),
		),
		distribution: WanderingAcaciaCell::distribution(),
	}
}

/// Ordered wandering-acacia varietals ([RFC-183 §3.4.7.16]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderingAcaciaCell {
	WanderingHighBush,
	DryWanderingSopesBanyan,
	WanderingVaseTree,
	WanderingPenmarchTorch,
	WanderingKamakuraTorch,
}

/// Typed authored geometry for one wandering-acacia varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WanderingAcaciaItem {
	HighBush(&'static WanderingAcaciaHighBush),
	Sope(&'static WanderingAcaciaBanyan),
	VaseTree(&'static WanderingAcaciaVaseTree),
	PenmarchTorch(&'static WanderingAcaciaTorch),
	KamakuraTorch(&'static WanderingAcaciaTorch),
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Sope's Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Vase Tree accent.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one wandering torch form.
#[derive(Debug, Clone, PartialEq)]
pub struct WanderingAcaciaTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const WANDERING_HIGH_BUSH: WanderingAcaciaHighBush = WanderingAcaciaHighBush {
	height: UnitRange::new(5.0, 15.0),
	shoot_count: 5..=16,
	branch_depth: 2..=4,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.45, 0.72),
};

const DRY_WANDERING_SOPE: WanderingAcaciaBanyan = WanderingAcaciaBanyan {
	height: UnitRange::new(5.0, 20.0),
	stalk_radius: UnitRange::new(0.14, 0.38),
	canopy_spread: UnitRange::new(2.5, 7.0),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_VASE_TREE: WanderingAcaciaVaseTree = WanderingAcaciaVaseTree {
	height: UnitRange::new(4.0, 8.0),
	stalk_radius: UnitRange::new(0.22, 0.48),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_PENMARCH_TORCH: WanderingAcaciaTorch = WanderingAcaciaTorch {
	height: UnitRange::new(5.0, 8.0),
	stalk_radius: UnitRange::new(0.14, 0.34),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const WANDERING_KAMAKURA_TORCH: WanderingAcaciaTorch = WanderingAcaciaTorch {
	height: UnitRange::new(5.0, 8.0),
	stalk_radius: UnitRange::new(0.12, 0.30),
	canopy_spread: UnitRange::new(0.5, 1.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const WANDERING_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const WANDERING_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const DRY_SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_banyan_bark", "tan_bark"),
	PaletteSlot::new("red_brown", "dark_bark"),
]);

const DRY_SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("deep_green", "dry_green"),
]);

const WANDERING_VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "acacia_bark"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const WANDERING_VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const WANDERING_PENMARCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("dry_bark", "dark_bark"),
]);

const WANDERING_PENMARCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("flower_yellow", "yellow_green"),
	PaletteSlot::new("olive_green", "dry_green"),
]);

const WANDERING_KAMAKURA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "orange_bark"),
	PaletteSlot::new("acacia_bark", "dark_bark"),
]);

const WANDERING_KAMAKURA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "olive_green"),
	PaletteSlot::new("pale_green", "dry_green"),
]);

impl WanderingAcaciaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.55`; the `None` weight of `37.0` puts the placed share at
	/// `3.55 / 40.55 ≈ 0.088`, mid RFC `DENSITY_RANGE` (`0.03..0.12`).
	pub fn distribution() -> GroveDistribution<Self> {
		let wandering_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.66));
		let dry_sope =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let wandering_vase =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.62));
		let wandering_penmarch =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.60));
		let wandering_kamakura =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		GroveDistribution::new(vec![
			GroveBucket::none(37.0),
			GroveBucket::placed(5.0, wandering_bush, Self::WanderingHighBush),
			GroveBucket::placed(1.0, dry_sope, Self::DryWanderingSopesBanyan),
			GroveBucket::placed(0.25, wandering_vase, Self::WanderingVaseTree),
			GroveBucket::placed(0.18, wandering_penmarch, Self::WanderingPenmarchTorch),
			GroveBucket::placed(0.12, wandering_kamakura, Self::WanderingKamakuraTorch),
		])
	}

	pub fn item(self) -> WanderingAcaciaItem {
		match self {
			Self::WanderingHighBush => WanderingAcaciaItem::HighBush(&WANDERING_HIGH_BUSH),
			Self::DryWanderingSopesBanyan => WanderingAcaciaItem::Sope(&DRY_WANDERING_SOPE),
			Self::WanderingVaseTree => WanderingAcaciaItem::VaseTree(&WANDERING_VASE_TREE),
			Self::WanderingPenmarchTorch => {
				WanderingAcaciaItem::PenmarchTorch(&WANDERING_PENMARCH_TORCH)
			}
			Self::WanderingKamakuraTorch => {
				WanderingAcaciaItem::KamakuraTorch(&WANDERING_KAMAKURA_TORCH)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::WanderingHighBush => WANDERING_BUSH_STICK_MIX,
			Self::DryWanderingSopesBanyan => DRY_SOPE_STICK_MIX,
			Self::WanderingVaseTree => WANDERING_VASE_STICK_MIX,
			Self::WanderingPenmarchTorch => WANDERING_PENMARCH_STICK_MIX,
			Self::WanderingKamakuraTorch => WANDERING_KAMAKURA_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::WanderingHighBush => WANDERING_BUSH_CANOPY_MIX,
			Self::DryWanderingSopesBanyan => DRY_SOPE_CANOPY_MIX,
			Self::WanderingVaseTree => WANDERING_VASE_CANOPY_MIX,
			Self::WanderingPenmarchTorch => WANDERING_PENMARCH_CANOPY_MIX,
			Self::WanderingKamakuraTorch => WANDERING_KAMAKURA_CANOPY_MIX,
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
		let dist = WanderingAcaciaCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 37.0);
		assert_eq!(dist.buckets[1].item, Some(WanderingAcaciaCell::WanderingHighBush));
		assert_eq!(dist.buckets[1].weight, 5.0);
		assert_eq!(dist.buckets[2].item, Some(WanderingAcaciaCell::DryWanderingSopesBanyan));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(WanderingAcaciaCell::WanderingVaseTree));
		assert_eq!(dist.buckets[3].weight, 0.25);
		assert_eq!(dist.buckets[4].item, Some(WanderingAcaciaCell::WanderingPenmarchTorch));
		assert_eq!(dist.buckets[4].weight, 0.18);
		assert_eq!(dist.buckets[5].item, Some(WanderingAcaciaCell::WanderingKamakuraTorch));
		assert_eq!(dist.buckets[5].weight, 0.12);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = WanderingAcaciaCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let WanderingAcaciaItem::HighBush(bush) = WanderingAcaciaCell::WanderingHighBush.item()
		else {
			anyhow::bail!("expected wandering high bush item");
		};
		assert_eq!(bush.height, UnitRange::new(5.0, 15.0));
		assert_eq!(bush.leaf_radius, UnitRange::new(0.45, 0.72));
		assert_eq!(bush.radial_strength, SPARSE_PROJECTION_RADIAL);

		let WanderingAcaciaItem::Sope(sope) = WanderingAcaciaCell::DryWanderingSopesBanyan.item()
		else {
			anyhow::bail!("expected dry wandering sope item");
		};
		assert_eq!(sope.height, UnitRange::new(5.0, 20.0));
		assert_eq!(sope.descender_density, SPARSE_DESCENDER_DENSITY);

		let WanderingAcaciaItem::VaseTree(vase) = WanderingAcaciaCell::WanderingVaseTree.item()
		else {
			anyhow::bail!("expected wandering vase item");
		};
		assert_eq!(vase.height, UnitRange::new(4.0, 8.0));
		assert_eq!(vase.canopy_density, SPARSE_CANOPY_DENSITY);

		let WanderingAcaciaItem::PenmarchTorch(torch) =
			WanderingAcaciaCell::WanderingPenmarchTorch.item()
		else {
			anyhow::bail!("expected wandering penmarch item");
		};
		assert_eq!(torch.height, UnitRange::new(5.0, 8.0));
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = WanderingAcaciaCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let bush = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(WanderingAcaciaCell::WanderingHighBush))
			.ok_or_else(|| anyhow::anyhow!("missing wandering high bush bucket"))?;
		assert_eq!(bush.constraints.steepness.end, 0.66);

		let sope = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(WanderingAcaciaCell::DryWanderingSopesBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing dry wandering sope bucket"))?;
		assert_eq!(sope.constraints.steepness.end, 0.58);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_dry_sope_but_falls_through_to_vase() -> Result<()> {
		let prepared = WanderingAcaciaCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
		let sope_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&moderate,
		);
		match sope_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, WanderingAcaciaCell::DryWanderingSopesBanyan);
			}
			other => {
				anyhow::bail!("expected DryWanderingSopesBanyan on moderate slope, got {other:?}")
			}
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
		let steep_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, WanderingAcaciaCell::WanderingVaseTree);
			}
			other => anyhow::bail!(
				"expected fall-through to WanderingVaseTree on steep slope, got {other:?}"
			),
		}
		let bush_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match bush_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, WanderingAcaciaCell::WanderingHighBush);
			}
			other => anyhow::bail!("expected WanderingHighBush on steep slope, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			WanderingAcaciaCell::WanderingHighBush,
			WanderingAcaciaCell::DryWanderingSopesBanyan,
			WanderingAcaciaCell::WanderingVaseTree,
			WanderingAcaciaCell::WanderingPenmarchTorch,
			WanderingAcaciaCell::WanderingKamakuraTorch,
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
