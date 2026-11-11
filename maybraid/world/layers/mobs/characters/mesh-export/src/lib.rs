//! Assemble a character in-process and write a mesh-only Wavefront OBJ for DCC import.

pub mod export;
pub mod obj_text;
pub mod skin;
pub mod uv;
pub mod validate;

pub use export::{
	apply_chart_uvs, assembly_block_reason, assembly_ready, assets_root, collect_groups,
	write_import_zip, write_obj_bundle, ExportReport, MeshGroup, ObjDocument,
};
pub use uv::{Triangle2, Triangle3, UnwrappedMesh, UvIsland};
pub use validate::{ExportCheck, MeshAudit};
