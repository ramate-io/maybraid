//! Assemble a character in-process and write a Wavefront OBJ with per-component textures.

pub mod export;
pub mod obj_text;
pub mod skin;
pub mod texture;
pub mod uv;
pub mod validate;

pub use export::{
	apply_chart_uvs, assembly_block_reason, assembly_ready, assets_root, collect_groups,
	write_import_zip, write_obj_bundle, write_obj_bundle_with, ExportReport, MeshGroup,
	MtlDocument, ObjDocument,
};
pub use texture::{TextureConfig, DEFAULT_TEXTURE_RESOLUTION};
pub use uv::{Triangle2, Triangle3, UnwrappedMesh, UvIsland};
pub use validate::{ExportCheck, MeshAudit, MtlTable};
