//! Stalk and ball-stick trees for Chico vegetation.
pub use chico_sdf;

pub mod date_palm;
pub mod waialea_palm;
pub mod liams_conifer;
pub mod skipped_mesh_material;
pub mod sopes_banyan;

pub use date_palm::DatePalmStd;
pub use waialea_palm::WaialeaPalmStd;
pub use liams_conifer::LiamsConiferStd;
pub use skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedMeshMaterial, SkippedStickMeshMaterial,
};
pub use sopes_banyan::SopesBanyanStd;
