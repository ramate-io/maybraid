//! GPU-driven tree canopies: trunk, limbs, branch skeleton, and layered foliage.
//!
//! CPU patches own compact plant / branch / leaf descriptors. Compute selects
//! stable LOD subsets (topology-preserving prune), then wood tubes and layered
//! teardrop canopy cards are generated in the vertex shader.
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

pub use bounds::TreeBounds;
pub use diagnostics::TreeFieldStats;
pub use field::{TreeField, TreeFieldSettings};
pub use host::TreePatchLod;
pub use layout::{
	BranchDescriptor, TreePatchHeader, LeafDescriptor, PlantDescriptor, WoodLod, BRANCH_DESCRIPTOR_STRIDE,
	TREE_LOD_COUNT, TREE_LOD_FAR, TREE_LOD_MID, TREE_LOD_NEAR, LEAF_DESCRIPTOR_STRIDE, PARENT_NONE,
	PLANT_DESCRIPTOR_STRIDE,
};
pub use patch::{TreePatch, TreeSample};
pub use plugin::TreeFieldPlugin;
pub use recipe::{TreeForm, TreeRecipe};
pub use select::{TreeSelector, CompactedTreeLists};
pub use template::{LeafTemplate, WoodTemplate};
pub use topology::{
	CanopyLayer, TreePlant, MAX_BRANCH_DEPTH, MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT,
};
pub use vegetation_grass_field::{GrassGpuCapability, GrassGpuMode, WorldFrustum};
pub use vegetation_shaders::wesl::{GrassViewUniform, VegetationPalette};

/// Workgroup size for the selection compute passes.
pub const SELECT_WORKGROUP: u32 = 64;
