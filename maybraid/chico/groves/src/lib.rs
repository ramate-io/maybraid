//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).

pub mod grove;

pub mod braid_grass;

pub use grove::{
	biased_sample, candidate_position, cell_origin, sample_cell_params, Bucket, CellGrove,
	ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome, GroveDistribution,
	GroveDistributionBuilder, GroveNoiseConfig, GroveParamRanges, PaletteColor, PaletteMix,
	PaletteSlot, PlacementConstraints, PreparedGroveDistribution, SampledCellParams,
	TerrainSample, WithPaletteMix,
};

pub use braid_grass::{BraidGrass, BraidGrassCell, BraidGrassGrove};
