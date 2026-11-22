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
pub use deform::MAX_ATTACH_HOPS;
pub use diagnostics::TreeFieldStats;
pub use field::{TreeField, TreeFieldSettings};
pub use gpu::{requested_world_tree_mode, supports_world_tree_field};
pub use host::TreePatchLod;
pub use layout::{
	BRANCH_DESCRIPTOR_STRIDE, BranchDescriptor, LEAF_DESCRIPTOR_STRIDE, LeafCloseStyle,
	LeafDescriptor, PARENT_NONE, PLANT_DESCRIPTOR_STRIDE, PlantDescriptor, TREE_LOD_COUNT,
	TREE_LOD_FAR, TREE_LOD_MID, TREE_LOD_NEAR, TreePatchHeader, WoodLod,
};
pub use patch::{TreePatch, TreeSample};
pub use plugin::{TreeFieldPlugin, TreeGrove};
pub use recipe::{TreeForm, TreeRecipe};
pub use select::{CompactedTreeLists, TreeSelector};
pub use template::{LeafTemplate, WoodTemplate};
pub use topology::{
	CanopyLayer, MAX_BRANCH_DEPTH, MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT, TreePlant,
};
pub use vegetation_grass_field::{GrassGpuCapability, GrassGpuMode, WorldFrustum};
pub use vegetation_shaders::wesl::{GrassViewUniform, VegetationPalette};

/// Workgroup size for the selection compute passes.
pub const SELECT_WORKGROUP: u32 = 64;
