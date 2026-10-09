//! LOD spatial identifiers and storage versions.
//!
//! Generation and presentation run on [`crate::hcsg`]. Scene / refresh
//! runtime is [`crate::scene`].

mod id;
mod version;

#[cfg(test)]
pub mod tests;

pub use crate::scene::{
	closest_available_lod_level, cull_bands_with_adjacent_depth, cull_named_from_factor,
	cull_non_adjacent_bands, cull_offset_bands, cull_offset_bands_from_factor, named_band_index,
	named_band_progress, LodSceneCull, LodSceneCulls, LodSceneLevel, QuantizedDistance, SceneChunk,
	DEFAULT_CHUNK_WEIGHT, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use crate::scene::{LodScene, LodSceneStatus, SemanticLodScene, VisualLodScene};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId};
pub use version::Version;
