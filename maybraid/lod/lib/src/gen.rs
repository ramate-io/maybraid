//! LOD spatial identifiers and storage indexing.
//!
//! Generation and presentation run on [`crate::hcsg::shared`]. Scene / refresh
//! runtime is [`crate::scene`]; marshalling is [`crate::presentation`].

mod id;
mod spatial_index;

#[cfg(test)]
pub mod tests;

pub use crate::presentation::{
	LodScene, LodSceneStatus, RegionPresenter, SemanticLodScene, VisualLodScene,
};
pub use crate::scene::{
	closest_available_lod_level, cull_bands_with_adjacent_depth, cull_named_from_factor,
	cull_non_adjacent_bands, cull_offset_bands, cull_offset_bands_from_factor, named_band_index,
	named_band_progress, LodSceneCull, LodSceneCulls, LodSceneLevel, QuantizedDistance, SceneChunk,
	DEFAULT_CHUNK_WEIGHT, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId, StorageStatus, TrackedId};
pub use spatial_index::{SpatialIndex, Version};
