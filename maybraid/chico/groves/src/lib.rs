//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).

pub mod grove;

pub mod understory;
pub mod tufts;

/// Back-compat re-export of [`understory::braid_grass`].
pub use understory::braid_grass as braid_grass;

#[cfg(feature = "render")]
pub mod skipped_mesh_material;

pub use grove::{
	parse_variant_weights, Bucket, CellGrove, CellXzOffset, DEFAULT_GROVE_EXTENT_XZ,
	FlatTerrainSample, ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome, GroveCellPlacement,
	GroveDistribution, GroveExtent, GroveFrontend, GroveNoiseConfig, GrovePlacedCell,
	GrovePlacementRanges, PaletteColor, PaletteMix, PaletteSlot, PlacementConstraints,
	PreparedGroveDistribution, SampledCellParams, TerrainSample, VariantWeightOverrides,
	WithPalette,
};

#[cfg(feature = "render")]
pub use grove::placement_noise;

pub use understory::braid_grass::{BraidGrassCell, BraidGrassClump, BraidGrassDefinition};

pub use tufts::tropical_tufts::{
	TropicalPalmBush, TropicalTuftClump, TropicalTuftsCell, TropicalTuftsDefinition,
};

#[cfg(feature = "render")]
pub use understory::braid_grass::{BraidGrass, BraidGrassStd};

#[cfg(feature = "render")]
pub use tufts::tropical_tufts::{TropicalTufts, TropicalTuftsStd};
