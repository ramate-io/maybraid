//! LOD spatial generation (storage + materialization).
//!
//! Phases and submodules:
//!
//! - [`spatial_index`]: storage truth. Values, bounds, and [`Version`] stamps
//!   by [`Id`]. Never generates, never presents.
//! - [`generation`]: pure materialization. [`GenerationScheme`] defines
//!   origins, building, and descendants per type; the single blanket lift to
//!   [`GeneratingSpatialIndex`] recurses the whole tree with no scene
//!   side effects.
//! - [`keep`]: impulse-queue liveness. Pending generate/present ids drop when
//!   their origin cell leaves the keep AABB plus per-channel slack
//!   ([`LodGenerateKeepRegion::slack_xz`] / [`crate::LodPresentKeepRegion::slack_xz`]).
//!
//! Presentation is [`crate::presentation`] (including its Bevy present
//! plugins). Scene / refresh runtime is [`crate::scene`]. Generate plugins
//! live in [`runtime`] and do not load the scene stack. Compatibility
//! re-exports of scene/presentation *types* are kept here for existing
//! `lod::gen::…` call sites.

mod generation;
mod id;
mod keep;
mod runtime;
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
pub use generation::{GeneratingSpatialIndex, GenerationScheme, MaterializeStatus};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId, StorageStatus, TrackedId};
pub use keep::{
	expand_keep_xz, expire_pending_outside_keep, id_lives_in_keep, id_xz_distance2,
	keep_region_changed, QUEUE_KEEP_SLACK_XZ,
};
pub use runtime::{
	drain_lod_generate, produce_lod_generate_regions, LodGenerateBudget, LodGenerateKeepRegion,
	LodGeneratePlugin, LodGenerateQueue, LodGenerateRegion, LodGenerateRegionPlugin,
	LodGenerateSystems, LodGenerated,
};
pub use spatial_index::{SpatialIndex, Version};
