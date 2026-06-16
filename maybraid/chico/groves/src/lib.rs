//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).

pub mod grove;

pub mod braid_grass;
pub mod common_tufts;
pub mod low_bush;
pub mod monster_grass;
pub mod riverine_green;
pub mod tropical_tufts;
pub mod wild_grass;

#[cfg(feature = "render")]
pub mod skipped_mesh_material;

pub use grove::{
	cell_center, parse_variant_weights, parse_vec2_csv, parse_vec3_csv, placement_noise,
	FlatTerrainSample, ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome, GroveDefinition,
	GroveDistribution, GroveExtent, GroveFrontend, GrovePlacedCell, GrovePlacementRanges,
	PaletteColor, PaletteMix, PaletteSlot, PlacementConstraints, PlacementSample,
	PreparedGroveDistribution, TerrainSample, VariantWeightOverrides, DEFAULT_GROVE_EXTENT_XZ,
};

#[cfg(feature = "render")]
pub use grove::{patch_spawned_leaf_material, resolve_palette_color, WithPalette};

pub use braid_grass::{BraidGrassCell, BraidGrassClump, BraidGrassItem, BraidSpearClump};
pub use common_tufts::{CommonTuftClump, CommonTuftsCell, CommonTuftsItem};
pub use low_bush::{LowBushBush, LowBushCell, LowBushItem};
pub use monster_grass::{MonsterGrassCell, MonsterGrassClump, MonsterGrassItem};
pub use riverine_green::{RiverineGreenBush, RiverineGreenCell, RiverineGreenItem};
pub use tropical_tufts::{
	TropicalPalmBush, TropicalTuftClump, TropicalTuftsCell, TropicalTuftsItem,
};
pub use wild_grass::{WildGrassCell, WildGrassClump, WildGrassItem};

#[cfg(feature = "render")]
pub use braid_grass::{BraidGrass, BraidGrassStd};
#[cfg(feature = "render")]
pub use common_tufts::{CommonTufts, CommonTuftsStd};
#[cfg(feature = "render")]
pub use low_bush::{LowBush, LowBushStd};
#[cfg(feature = "render")]
pub use monster_grass::{MonsterGrass, MonsterGrassStd};
#[cfg(feature = "render")]
pub use riverine_green::{RiverineGreen, RiverineGreenStd};
#[cfg(feature = "render")]
pub use tropical_tufts::{TropicalTufts, TropicalTuftsStd};
#[cfg(feature = "render")]
pub use wild_grass::{WildGrass, WildGrassStd};
