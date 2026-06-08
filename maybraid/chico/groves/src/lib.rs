//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).

pub mod grove;

pub mod braid_grass;

#[cfg(feature = "render")]
pub mod skipped_mesh_material;

pub use grove::{
	biased_sample, candidate_position, cell_origin, parse_variant_weights, braid_grass_preview_cells,
	preview_cell_grid, sample_cell_params, Bucket, CellGrove, FlatTerrainSample, ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome,
	GroveDistribution, GroveDistributionBuilder, GroveFrontend, GroveNoiseConfig, GroveParamRanges,
	PaletteColor, PaletteMix, PaletteSlot, PlacementConstraints, PreparedGroveDistribution,
	SampledCellParams, TerrainSample, VariantWeightOverrides, WithPaletteMix,
};

pub use braid_grass::{BraidGrassCell, BraidGrassClump, BraidGrassDefinition};

#[cfg(feature = "render")]
pub use braid_grass::{BraidGrass, BraidGrassRenderRule, BraidGrassStd};

#[cfg(feature = "render")]
pub use grove::{GrovePlacedCell, GroveRenderHelper, GroveRenderRule};
