//! Common (sub)component assemblies for Chico vegetation ([#226](https://github.com/ramate-io/maybraid/issues/226)).
pub mod jungle_growth;
pub mod jungle_storybook_canopy;
pub mod braid_oak_canopy;
pub mod skipped_mesh_material;

pub use jungle_growth::{JungleGrowth, JungleGrowthShape};
pub use jungle_storybook_canopy::JungleStorybookCanopyFoliage;
pub use braid_oak_canopy::BraidOakCanopyFoliage;
pub use skipped_mesh_material::{
	SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
};
