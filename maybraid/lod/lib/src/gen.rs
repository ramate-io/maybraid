//! LOD spatial generation and presentation.
//!
//! Three phases, three submodules:
//!
//! - [`spatial_index`]: storage truth. Values, bounds, and [`Version`] stamps
//!   by [`Id`]. Never generates, never presents.
//! - [`generation`]: pure materialization. [`GenerationScheme`] defines
//!   origins, building, and descendants per type; the single blanket lift to
//!   [`GeneratingSpatialIndex`] recurses the whole tree with no scene
//!   side effects.
//! - [`presentation`]: a separate pass after generation. [`RegionPresenter`]
//!   diffs storage versions against what it has presented, handles new or
//!   changed ids, then removes stale ones.
//!
//! Pitfalls avoided:
//!
//! - Spawning from `insert()` or from inside generation is too implicit; it
//!   couples visual effects to data recursion and can present descendants
//!   that were only meant to be indexed.
//! - Transient "created" events between the phases would need a commit
//!   protocol (who drains, when, how many consumers). Version stamps carry
//!   the same information as plain data.

mod generation;
mod id;
mod presentation;
mod spatial_index;

#[cfg(test)]
mod tests;

pub use crate::lod_level::{LodSceneLevel, QuantizedDistance};
pub use generation::{GeneratingSpatialIndex, GenerationScheme, MaterializeStatus};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId, StorageStatus, TrackedId};
pub use crate::lod_cull::{
	cull_bands_with_adjacent_depth, cull_named_from_factor, cull_non_adjacent_bands,
	cull_offset_bands, cull_offset_bands_from_factor, named_band_index, named_band_progress,
	LodSceneCull, LodSceneCulls, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use presentation::{LodScene, LodSceneStatus, RegionPresenter};
pub use spatial_index::{SpatialIndex, Version};
