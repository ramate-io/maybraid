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
mod grove_catalog;
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
	CatalogOpQueue, GroveEncodeStats, GroveId, RangeArena, RangeSpan, ADMISSION_BUDGET,
};
pub use diagnostics::GrassFieldStats;
pub use field::{GrassField, GrassFieldSettings};
pub use grove_catalog::{GrassCatalogOp, GrassCatalogOps, GrassGroveCatalog};
pub use host::GrassPatchLod;
pub use patch::{GrassPatch, SurfaceSample};
pub use plugin::GrassFieldPlugin;
pub use select::{frustum_planes, CompactedLists, GrassSelector, WorldFrustum};
pub use species::GrassSpecies;
pub use template::BladeTemplate;
pub use vegetation_shaders::wesl::{
	BladeDescriptor, GeometryLod, GrassPatchHeader, GrassViewUniform, VegetationPalette,
	DRAW_INDEXED_INDIRECT_STRIDE, GRASS_LOD_CARD, GRASS_LOD_COUNT, GRASS_LOD_MID, GRASS_LOD_NEAR,
};

/// Workgroup size for the selection compute pass.
pub const SELECT_WORKGROUP: u32 = 64;
