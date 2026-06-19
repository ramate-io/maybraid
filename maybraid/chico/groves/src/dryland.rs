//! Dryland — very-low-density arid upper-canopy grove with Liam's Conifer and Vase Tree
//! ([RFC-183 §3.4.7.13], [#335](https://github.com/ramate-io/maybraid/issues/335)).
//!
//! Sparse dry highland canopy with evenly common Liam's Conifer and Vase Tree forms. Forest-layer
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
pub use render::{Dryland, DrylandStd};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);

/// Authored Dryland grove definition.
///
/// Cell footprint sits at the RFC midpoint (`35.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<DrylandCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(35.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-35.0, 35.0),
		),
		distribution: DrylandCell::distribution(),
	}
}

/// Ordered dryland varietals ([RFC-183 §3.4.7.13]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrylandCell {
	DrylandLiamsConifer,
	DrylandVaseTree,
}

/// Typed authored geometry for one dryland varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrylandItem {
	LiamsConifer(&'static DrylandLiamsConifer),
	VaseTree(&'static DrylandVaseTree),
}

/// Authored geometry ranges for one dry Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const DRYLAND_LIAMS: DrylandLiamsConifer = DrylandLiamsConifer {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.25, 0.50),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_VASE: DrylandVaseTree = DrylandVaseTree {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.34, 0.68),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_conifer_bark", "tan_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const DRYLAND_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "dusty_green"),
	PaletteSlot::new("deep_green", "olive_green"),
]);

const DRYLAND_VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "tan_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const DRYLAND_VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

impl DrylandCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.0`; the `None` weight of `24.7` puts the placed share at
	/// `2.0 / 26.7 ≈ 0.075`, mid RFC `DENSITY_RANGE` (`0.03..0.12`).
	pub fn distribution() -> GroveDistribution<Self> {
		let liams = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.82));
		let vase = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(24.7),
			GroveBucket::placed(1.0, liams, Self::DrylandLiamsConifer),
			GroveBucket::placed(1.0, vase, Self::DrylandVaseTree),
		])
	}

	pub fn item(self) -> DrylandItem {
		match self {
			Self::DrylandLiamsConifer => DrylandItem::LiamsConifer(&DRYLAND_LIAMS),
			Self::DrylandVaseTree => DrylandItem::VaseTree(&DRYLAND_VASE),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_STICK_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_CANOPY_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_CANOPY_MIX,
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
		let dist = DrylandCell::distribution();
		assert_eq!(dist.len(), 3);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.7);
		assert_eq!(dist.buckets[1].item, Some(DrylandCell::DrylandLiamsConifer));
		assert_eq!(dist.buckets[1].weight, 1.0);
		assert_eq!(dist.buckets[2].item, Some(DrylandCell::DrylandVaseTree));
		assert_eq!(dist.buckets[2].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = DrylandCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.03..=0.12).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let DrylandItem::LiamsConifer(liams) = DrylandCell::DrylandLiamsConifer.item() else {
			anyhow::bail!("expected liams item");
		};
		assert_eq!(liams.height, UnitRange::new(10.0, 20.0));
		assert_eq!(liams.canopy_density, SPARSE_CANOPY_DENSITY);

		let DrylandItem::VaseTree(vase) = DrylandCell::DrylandVaseTree.item() else {
			anyhow::bail!("expected vase item");
		};
		assert_eq!(vase.height, UnitRange::new(10.0, 20.0));
		assert_eq!(vase.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = DrylandCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let liams = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DrylandCell::DrylandLiamsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing liams bucket"))?;
		assert_eq!(liams.constraints.steepness.end, 0.82);

		let vase = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DrylandCell::DrylandVaseTree))
			.ok_or_else(|| anyhow::anyhow!("missing vase bucket"))?;
		assert_eq!(vase.constraints.steepness.end, 0.70);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_vase_but_allows_liams() -> Result<()> {
		let prepared =
			DrylandCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
		let vase_outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, &moderate);
		match vase_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, DrylandCell::DrylandVaseTree);
			}
			other => anyhow::bail!("expected DrylandVaseTree on moderate slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.75 };
		let liams_outcome = prepared.select_from(1, Vec3::new(5.0, 0.40, 5.0), 1.0, &steep);
		match liams_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, DrylandCell::DrylandLiamsConifer);
			}
			other => anyhow::bail!("expected DrylandLiamsConifer on steep slope, got {other:?}"),
		}
		match prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, &steep) {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, DrylandCell::DrylandVaseTree);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [DrylandCell::DrylandLiamsConifer, DrylandCell::DrylandVaseTree] {
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(280.0, 1.0, 280.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
