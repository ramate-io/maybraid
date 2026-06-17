//! Tropical Undergrowth — well-known moderate-to-dense hybrid understory grove
//! ([RFC-183 §3.4.5.5], [#315](https://github.com/ramate-io/maybraid/issues/315)).
//!
//! Mixes bright/deep tufts (mostly as patches), small palm bushes, and rare mini SBS-tree forms.
//! Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{TropicalUndergrowth, TropicalUndergrowthStd};

/// Authored Tropical Undergrowth grove definition.
///
/// Cell footprint sits at the RFC midpoint (`5.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalUndergrowthCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(5.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-5.0, 5.0)),
		distribution: TropicalUndergrowthCell::distribution(),
	}
}

/// Ordered tropical-undergrowth varietals ([RFC-183 §3.4.5.5]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalUndergrowthCell {
	BrightTuft,
	DeepTuft,
	SmallPalmBush,
	MiniRoryHeadTrained,
	MiniVaseTree,
	MiniSparseStorybook,
	MiniPenmarchTorch,
	MiniKamakuraTorch,
	MiniTorchTree,
	BrightTuftPatch,
	DeepTuftPatch,
}

/// Typed authored geometry for one tropical-undergrowth varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalUndergrowthItem {
	Tuft(&'static TropicalUndergrowthTuft),
	Patch(&'static GroveTuftPatch<TropicalUndergrowthTuft>),
	PalmBush(&'static TropicalUndergrowthPalm),
	RoryHead(&'static TropicalUndergrowthRoryHead),
	VaseTree(&'static TropicalUndergrowthVaseTree),
	Storybook(&'static TropicalUndergrowthStorybook),
	PenmarchTorch(&'static TropicalUndergrowthTorch),
	KamakuraTorch(&'static TropicalUndergrowthTorch),
	TorchTree(&'static TropicalUndergrowthTorch),
}

/// Authored geometry ranges for one tropical-undergrowth tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTuft {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthPalm {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

/// Authored geometry ranges for one mini Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthRoryHead {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one mini Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini torch form (Penmarch, Kamakura, or generic torch tree).
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthTorch {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Authored geometry ranges for one mini Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalUndergrowthStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 6..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

const BRIGHT_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.30, 1.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DEEP_TUFT: TropicalUndergrowthTuft = TropicalUndergrowthTuft {
	height: UnitRange::new(0.40, 0.90),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const BRIGHT_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: BRIGHT_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const DEEP_TUFT_PATCH: GroveTuftPatch<TropicalUndergrowthTuft> = GroveTuftPatch {
	clump: DEEP_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const SMALL_PALM_BUSH: TropicalUndergrowthPalm = TropicalUndergrowthPalm {
	height: UnitRange::new(0.50, 1.40),
	frond_count: 5..=9,
	frond_length: UnitRange::new(0.25, 0.70),
	crown_spread: UnitRange::new(0.35, 0.90),
};

const MINI_RORY_HEAD: TropicalUndergrowthRoryHead = TropicalUndergrowthRoryHead {
	height: UnitRange::new(0.80, 1.80),
	stalk_radius: UnitRange::new(0.020, 0.030),
	canopy_spread: UnitRange::new(0.50, 1.20),
	canopy_density: UnitRange::new(0.0, 1.0),
};

const MINI_VASE_TREE: TropicalUndergrowthVaseTree = TropicalUndergrowthVaseTree {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.025, 0.035),
	canopy_spread: UnitRange::new(0.70, 1.50),
};

const MINI_STORYBOOK: TropicalUndergrowthStorybook = TropicalUndergrowthStorybook {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.025, 0.035),
	canopy_spread: UnitRange::new(0.60, 1.40),
};

const MINI_TORCH_TREE: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.20),
	stalk_radius: UnitRange::new(0.025, 0.035),
	canopy_spread: UnitRange::new(0.55, 1.20),
};

const MINI_PENMARCH_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.20, 2.50),
	stalk_radius: UnitRange::new(0.025, 0.035),
	canopy_spread: UnitRange::new(0.65, 1.40),
};

const MINI_KAMAKURA_TORCH: TropicalUndergrowthTorch = TropicalUndergrowthTorch {
	height: UnitRange::new(1.00, 2.30),
	stalk_radius: UnitRange::new(0.025, 0.035),
	canopy_spread: UnitRange::new(0.60, 1.30),
};

const BRIGHT_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("bright_green", "lime_green"),
	PaletteSlot::new("lush_green", "fresh_green"),
	PaletteSlot::new("yellow_green", "light_green"),
]);

const DEEP_TUFT_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "emerald_green"),
	PaletteSlot::new("dark_green", "wet_green"),
	PaletteSlot::new("blue_green", "bright_green"),
]);

const PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("green_stem", "wet_brown"),
]);

const PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("blue_green", "wet_green"),
	PaletteSlot::new("yellow_green", "lime_green"),
]);

const VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "tropical_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("dark_green", "emerald_green"),
	PaletteSlot::new("flower_white", "fresh_green"),
]);

const STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "light_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("blue_green", "yellow_green"),
]);

const TORCH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("tropical_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);

const TORCH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("yellow_green", "warm_yellow"),
	PaletteSlot::new("lime_green", "fresh_green"),
]);

impl TropicalUndergrowthCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.98` (RFC relative proportions plus rare torch companions); the
	/// `None` weight of `9.0` puts the placed share at `5.98 / 14.98 ≈ 0.40`, mid RFC
	/// `DENSITY_RANGE` (`0.22..0.58`).
	pub fn distribution() -> GroveDistribution<Self> {
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.70));
		let palm = PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.60));
		let mini_tree =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(9.0),
			GroveBucket::placed(0.4, lowland, Self::BrightTuft),
			GroveBucket::placed(0.3, lowland, Self::DeepTuft),
			GroveBucket::placed(1.0, palm, Self::SmallPalmBush),
			GroveBucket::placed(0.85, lowland, Self::MiniRoryHeadTrained),
			GroveBucket::placed(0.20, mini_tree, Self::MiniVaseTree),
			GroveBucket::placed(0.15, mini_tree, Self::MiniSparseStorybook),
			GroveBucket::placed(0.12, mini_tree, Self::MiniPenmarchTorch),
			GroveBucket::placed(0.08, mini_tree, Self::MiniKamakuraTorch),
			GroveBucket::placed(0.08, mini_tree, Self::MiniTorchTree),
			GroveBucket::placed(1.6, lowland, Self::BrightTuftPatch),
			GroveBucket::placed(1.2, lowland, Self::DeepTuftPatch),
		])
	}

	pub fn item(self) -> TropicalUndergrowthItem {
		match self {
			Self::BrightTuft => TropicalUndergrowthItem::Tuft(&BRIGHT_TUFT),
			Self::DeepTuft => TropicalUndergrowthItem::Tuft(&DEEP_TUFT),
			Self::SmallPalmBush => TropicalUndergrowthItem::PalmBush(&SMALL_PALM_BUSH),
			Self::MiniRoryHeadTrained => TropicalUndergrowthItem::RoryHead(&MINI_RORY_HEAD),
			Self::MiniVaseTree => TropicalUndergrowthItem::VaseTree(&MINI_VASE_TREE),
			Self::MiniSparseStorybook => TropicalUndergrowthItem::Storybook(&MINI_STORYBOOK),
			Self::MiniPenmarchTorch => TropicalUndergrowthItem::PenmarchTorch(&MINI_PENMARCH_TORCH),
			Self::MiniKamakuraTorch => TropicalUndergrowthItem::KamakuraTorch(&MINI_KAMAKURA_TORCH),
			Self::MiniTorchTree => TropicalUndergrowthItem::TorchTree(&MINI_TORCH_TREE),
			Self::BrightTuftPatch => TropicalUndergrowthItem::Patch(&BRIGHT_TUFT_PATCH),
			Self::DeepTuftPatch => TropicalUndergrowthItem::Patch(&DEEP_TUFT_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::BrightTuft | Self::BrightTuftPatch => BRIGHT_TUFT_MIX,
			Self::DeepTuft | Self::DeepTuftPatch => DEEP_TUFT_MIX,
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_STICK_MIX,
			Self::MiniRoryHeadTrained => RORY_STICK_MIX,
			Self::MiniVaseTree => VASE_STICK_MIX,
			Self::MiniSparseStorybook => STORYBOOK_STICK_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_STICK_MIX
			}
			_ => PALM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallPalmBush => PALM_CANOPY_MIX,
			Self::MiniRoryHeadTrained => RORY_CANOPY_MIX,
			Self::MiniVaseTree => VASE_CANOPY_MIX,
			Self::MiniSparseStorybook => STORYBOOK_CANOPY_MIX,
			Self::MiniPenmarchTorch | Self::MiniKamakuraTorch | Self::MiniTorchTree => {
				TORCH_CANOPY_MIX
			}
			_ => BRIGHT_TUFT_MIX,
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
		let dist = TropicalUndergrowthCell::distribution();
		assert_eq!(dist.len(), 12);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 9.0);
		assert_eq!(dist.buckets[1].item, Some(TropicalUndergrowthCell::BrightTuft));
		assert_eq!(dist.buckets[1].weight, 0.4);
		assert_eq!(dist.buckets[2].item, Some(TropicalUndergrowthCell::DeepTuft));
		assert_eq!(dist.buckets[2].weight, 0.3);
		assert_eq!(dist.buckets[3].item, Some(TropicalUndergrowthCell::SmallPalmBush));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(TropicalUndergrowthCell::MiniRoryHeadTrained));
		assert_eq!(dist.buckets[4].weight, 0.85);
		assert_eq!(dist.buckets[5].item, Some(TropicalUndergrowthCell::MiniVaseTree));
		assert_eq!(dist.buckets[5].weight, 0.20);
		assert_eq!(dist.buckets[6].item, Some(TropicalUndergrowthCell::MiniSparseStorybook));
		assert_eq!(dist.buckets[6].weight, 0.15);
		assert_eq!(dist.buckets[7].item, Some(TropicalUndergrowthCell::MiniPenmarchTorch));
		assert_eq!(dist.buckets[7].weight, 0.12);
		assert_eq!(dist.buckets[8].item, Some(TropicalUndergrowthCell::MiniKamakuraTorch));
		assert_eq!(dist.buckets[8].weight, 0.08);
		assert_eq!(dist.buckets[9].item, Some(TropicalUndergrowthCell::MiniTorchTree));
		assert_eq!(dist.buckets[9].weight, 0.08);
		assert_eq!(dist.buckets[10].item, Some(TropicalUndergrowthCell::BrightTuftPatch));
		assert_eq!(dist.buckets[10].weight, 1.6);
		assert_eq!(dist.buckets[11].item, Some(TropicalUndergrowthCell::DeepTuftPatch));
		assert_eq!(dist.buckets[11].weight, 1.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = TropicalUndergrowthCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.22..=0.58).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_tufts() -> Result<()> {
		let tuft_weight = |patch: bool| -> f32 {
			TropicalUndergrowthCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match cell.item() {
						TropicalUndergrowthItem::Tuft(_) => !patch,
						TropicalUndergrowthItem::Patch(_) => patch,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		assert!(
			tuft_weight(true) > 2.0 * tuft_weight(false),
			"patches should dominate tuft weight"
		);
		Ok(())
	}

	#[test]
	fn tuft_palm_and_tree_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			TropicalUndergrowthCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						(
							"tuft",
							TropicalUndergrowthItem::Tuft(_) | TropicalUndergrowthItem::Patch(_),
						) => true,
						("palm", TropicalUndergrowthItem::PalmBush(_)) => true,
						("rory", TropicalUndergrowthItem::RoryHead(_)) => true,
						("vase", TropicalUndergrowthItem::VaseTree(_)) => true,
						("story", TropicalUndergrowthItem::Storybook(_)) => true,
						(
							"torch",
							TropicalUndergrowthItem::PenmarchTorch(_)
							| TropicalUndergrowthItem::KamakuraTorch(_)
							| TropicalUndergrowthItem::TorchTree(_),
						) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let tuft = weight("tuft");
		let palm = weight("palm");
		let rory = weight("rory");
		let vase = weight("vase");
		let story = weight("story");
		let torch = weight("torch");
		assert!((tuft - 3.5).abs() < 1e-4, "expected tuft weight 3.5, got {tuft}");
		assert!((palm - 1.0).abs() < 1e-4, "expected palm weight 1.0, got {palm}");
		assert!((rory - 0.85).abs() < 1e-4, "expected rory weight 0.85, got {rory}");
		assert!((vase - 0.20).abs() < 1e-4, "expected vase weight 0.20, got {vase}");
		assert!((story - 0.15).abs() < 1e-4, "expected story weight 0.15, got {story}");
		assert!((torch - 0.28).abs() < 1e-4, "expected torch weight 0.28, got {torch}");
		Ok(())
	}

	#[test]
	fn tuft_geometry_follows_authored_bands() -> Result<()> {
		let TropicalUndergrowthItem::Tuft(bright) = TropicalUndergrowthCell::BrightTuft.item()
		else {
			anyhow::bail!("expected bright tuft item");
		};
		assert!(bright.height.start >= 0.30);
		assert!(bright.height.end <= 1.50);

		let TropicalUndergrowthItem::Tuft(deep) = TropicalUndergrowthCell::DeepTuft.item() else {
			anyhow::bail!("expected deep tuft item");
		};
		assert!(deep.height.start >= 0.40);
		assert!(deep.height.end <= 0.90);
		Ok(())
	}

	#[test]
	fn palm_and_mini_tree_geometry_follows_authored_bands() -> Result<()> {
		let TropicalUndergrowthItem::PalmBush(palm) = TropicalUndergrowthCell::SmallPalmBush.item()
		else {
			anyhow::bail!("expected palm item");
		};
		assert!(palm.height.start >= 0.50);
		assert!(palm.height.end <= 1.40);
		assert_eq!(palm.frond_count, 5..=9);

		let TropicalUndergrowthItem::RoryHead(rory) =
			TropicalUndergrowthCell::MiniRoryHeadTrained.item()
		else {
			anyhow::bail!("expected rory item");
		};
		assert!(rory.height.start >= 0.80);
		assert!(rory.height.end <= 1.80);
		assert!(rory.stalk_radius.start >= 0.020);
		assert!(rory.stalk_radius.end <= 0.030);
		assert!(rory.canopy_spread.start >= 0.50);
		assert!(rory.canopy_density.end <= 1.0);

		let TropicalUndergrowthItem::VaseTree(vase) = TropicalUndergrowthCell::MiniVaseTree.item()
		else {
			anyhow::bail!("expected vase item");
		};
		assert!(vase.height.start >= 1.00);
		assert!(vase.height.end <= 2.30);
		assert!(vase.stalk_radius.start >= 0.025);
		assert!(vase.canopy_spread.end <= 1.50);

		let TropicalUndergrowthItem::Storybook(story) =
			TropicalUndergrowthCell::MiniSparseStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert!(story.height.start >= 1.20);
		assert!(story.height.end <= 2.50);
		assert!(story.stalk_radius.end <= 0.035);
		assert!(story.canopy_spread.start >= 0.60);
		Ok(())
	}

	#[test]
	fn patch_wraps_bright_tuft_clump() -> Result<()> {
		let TropicalUndergrowthItem::Patch(patch) = TropicalUndergrowthCell::BrightTuftPatch.item()
		else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, BRIGHT_TUFT);
		assert!(*patch.clump_count.start() >= 3);
		assert!(patch.patch_extent_xz.start >= 1.0);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// SmallPalmBush (index 3) rejects steepness 0.65; first-fit falls to MiniRoryHeadTrained
		// (index 4), which allows steepness up to 0.70.
		let prepared = TropicalUndergrowthCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.65 };
		let outcome = prepared.select_from(3, Vec3::new(5.0, 0.35, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, TropicalUndergrowthCell::MiniRoryHeadTrained);
			}
			other => anyhow::bail!("expected MiniRoryHeadTrained fallback, got {other:?}"),
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
