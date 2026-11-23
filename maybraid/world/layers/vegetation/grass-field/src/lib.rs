//! GPU-driven moderate-height grass fields.
//!
//! CPU patches own surface samples and compact blade descriptors. A compute pass
//! selects stable density subsets, assigns geometry LOD, and writes indirect draw
//! arguments. Vertex shaders generate tapered ribbons from those descriptors.
//!
//! Shared math lives in [`vegetation_shaders::wesl`]. This crate owns patch
//! lifecycle, GPU buffers, and grass entry points. The existing terrain turf mat
//! and frond tuft path stay available.

mod bounds;
mod capability;
mod catalog;
mod diagnostics;
mod field;
mod gpu;
mod gpu_arena;
mod grove_catalog;
mod grove_cull;
mod grove_record;
mod grove_retain;
mod host;
mod patch;
mod plugin;
mod select;
mod shaders;
mod species;
mod template;

pub use bounds::GrassBounds;
pub use capability::{requested_vegetation_gpu_mode, GrassGpuCapability, GrassGpuMode};
pub use catalog::{
	select_dispatch_size, select_item_index, CatalogApply, CatalogOpQueue, GroveCatalogOp,
	GroveEncodeStats, GroveId, RangeArena, RangeSpan, ADMISSION_BUDGET,
	MAX_COMPUTE_WORKGROUPS_PER_DIM,
};
pub use diagnostics::GrassFieldStats;
pub use field::{GrassField, GrassFieldSettings};
pub use gpu_arena::{
	encode_arena_copies, grow_buffer_in_place, grow_list_buffers, grow_storage_buffer,
	stage_buffer_write, upload_visible_spans, ArenaBufferPlan, ArenaGrow, PendingBufferCopy,
	PendingBufferWrite, INITIAL_LIST_ITEMS,
};
pub use grove_catalog::{GrassCatalogOp, GrassCatalogOps, GrassGroveCatalog};
pub use grove_cull::{
	cull_grass_patches, cull_woody_groves, grass_patch_visible, grove_lod_band, map_work_index,
	woody_grove_visible, GROVE_BAND_HYSTERESIS_M,
};
pub use grove_record::{
	GroveVisibleSpan, VisibleGrove, WoodyGroveRecord, WOODY_GROVE_RECORD_STRIDE,
};
pub use grove_retain::{remember_bands, GroveRetainPolicy, GroveSubmit, RetainedGroveFrame};
pub use host::GrassPatchLod;
pub use patch::{GrassPatch, SurfaceSample};
pub use plugin::{collect_catalog_ops, GrassFieldPlugin};
pub use select::{
	aabb_nearest_distance, frustum_planes, CompactedLists, GrassSelector, WorldFrustum,
};
pub use species::GrassSpecies;
pub use template::BladeTemplate;
pub use vegetation_shaders::wesl::{
	BladeDescriptor, GeometryLod, GrassPatchHeader, GrassViewUniform, VegetationPalette,
	DRAW_INDEXED_INDIRECT_STRIDE, GRASS_LOD_CARD, GRASS_LOD_COUNT, GRASS_LOD_MID, GRASS_LOD_NEAR,
};

/// Workgroup size for the selection compute pass.
pub const SELECT_WORKGROUP: u32 = 64;
