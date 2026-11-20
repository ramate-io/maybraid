//! GPU-driven small woody bushes with attached foliage.
//!
//! CPU patches own compact plant / branch / leaf descriptors. Compute selects
//! stable LOD subsets (topology-preserving prune), then wood tubes and folded
//! leaves are generated in the vertex shader.
//!
//! Shared math lives in [`vegetation_shaders::wesl`]. Patch lifecycle, buffers,
//! and visibility follow [`vegetation_grass_field`]. Recipes stay in this crate.

mod bounds;
mod deform;
mod diagnostics;
mod field;
mod gpu;
mod host;
mod layout;
mod patch;
mod plugin;
mod recipe;
mod select;
mod shaders;
mod template;
mod topology;

pub use bounds::BushBounds;
pub use diagnostics::BushFieldStats;
pub use field::{BushField, BushFieldSettings};
pub use host::BushPatchLod;
pub use layout::{
	BranchDescriptor, BushPatchHeader, LeafDescriptor, PlantDescriptor, WoodLod, BRANCH_DESCRIPTOR_STRIDE,
	BUSH_LOD_COUNT, BUSH_LOD_FAR, BUSH_LOD_MID, BUSH_LOD_NEAR, LEAF_DESCRIPTOR_STRIDE, PARENT_NONE,
	PLANT_DESCRIPTOR_STRIDE,
};
pub use patch::{BushPatch, BushSample};
pub use plugin::BushFieldPlugin;
pub use recipe::BushRecipe;
pub use select::{BushSelector, CompactedBushLists};
pub use template::{LeafTemplate, WoodTemplate};
pub use topology::{BushPlant, MAX_BRANCH_DEPTH, MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT};
pub use vegetation_grass_field::{GrassGpuCapability, GrassGpuMode, WorldFrustum};
pub use vegetation_shaders::wesl::{GrassViewUniform, VegetationPalette};

/// Workgroup size for the selection compute passes.
pub const SELECT_WORKGROUP: u32 = 64;
