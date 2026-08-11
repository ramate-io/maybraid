//! LOD spatial generation (storage + materialization).
//!
//! Two phases, two submodules:
//!
//! - [`spatial_index`]: storage truth. Values, bounds, and [`Version`] stamps
//!   by [`Id`]. Never generates, never presents.
//! - [`generation`]: pure materialization. [`GenerationScheme`] defines
//!   origins, building, and descendants per type; the single blanket lift to
//!   [`GeneratingSpatialIndex`] recurses the whole tree with no scene
//!   side effects.
//!
//! Presentation is [`crate::presentation`]; scene / refresh runtime is
//! [`crate::scene`]. Compatibility re-exports of scene/presentation types are
//! kept here for existing `lod::gen::…` call sites.

mod generation;
mod id;
mod spatial_index;

#[cfg(test)]
pub mod tests;

pub use crate::presentation::{LodScene, LodSceneStatus, RegionPresenter};
pub use crate::scene::{
	closest_available_lod_level, cull_bands_with_adjacent_depth, cull_named_from_factor,
	cull_non_adjacent_bands, cull_offset_bands, cull_offset_bands_from_factor, named_band_index,
	named_band_progress, LodSceneCull, LodSceneCulls, LodSceneLevel, QuantizedDistance, SceneChunk,
	DEFAULT_CHUNK_WEIGHT, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use generation::{GeneratingSpatialIndex, GenerationScheme, MaterializeStatus};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId, StorageStatus, TrackedId};
pub use spatial_index::{SpatialIndex, Version};
