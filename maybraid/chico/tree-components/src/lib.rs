//! Common (sub)component assemblies for Chico vegetation ([#226](https://github.com/ramate-io/maybraid/issues/226)).
pub mod jungle_growth;
pub mod skipped_mesh_material;

pub use jungle_growth::{JungleGrowth, JungleGrowthShape};
pub use skipped_mesh_material::{
	SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
};
