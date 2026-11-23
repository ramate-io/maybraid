//! Bush compute / draw entry points. They import `vegetation_shaders::wesl::*`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::shader::load_shader_library;

pub const CULL_GROVES_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/cull_groves.wesl");
pub const PREFIX_SPANS_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/prefix_spans.wesl");
pub const SELECT_WOOD_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/select_wood.wesl");
pub const SELECT_FOLIAGE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/select_foliage.wesl");
pub const FINALIZE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/finalize.wesl");
pub const WOOD_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/wood.wesl");
pub const FOLIAGE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/foliage.wesl");

pub struct BushShadersPlugin;

impl Plugin for BushShadersPlugin {
	fn build(&self, app: &mut App) {
		load_shader_library!(app, "shaders/layout.wesl");
		embedded_asset!(app, "shaders/cull_groves.wesl");
		embedded_asset!(app, "shaders/prefix_spans.wesl");
		embedded_asset!(app, "shaders/select_wood.wesl");
		embedded_asset!(app, "shaders/select_foliage.wesl");
		embedded_asset!(app, "shaders/finalize.wesl");
		embedded_asset!(app, "shaders/wood.wesl");
		embedded_asset!(app, "shaders/foliage.wesl");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shader_refs_match_embedded_layout() -> anyhow::Result<()> {
		assert_eq!(SELECT_WOOD_SHADER, "embedded://vegetation_bush_field/shaders/select_wood.wesl");
		assert_eq!(WOOD_SHADER, "embedded://vegetation_bush_field/shaders/wood.wesl");
		assert_eq!(FOLIAGE_SHADER, "embedded://vegetation_bush_field/shaders/foliage.wesl");
		Ok(())
	}

	#[test]
	fn entry_points_import_wesl_library() -> anyhow::Result<()> {
		let wood = include_str!("shaders/wood.wesl");
		let foliage = include_str!("shaders/foliage.wesl");
		let select_wood = include_str!("shaders/select_wood.wesl");
		let select_foliage = include_str!("shaders/select_foliage.wesl");
		let cull = include_str!("shaders/cull_groves.wesl");
		let prefix = include_str!("shaders/prefix_spans.wesl");
		assert!(wood.contains("import vegetation_shaders::wesl::curve"));
		assert!(wood.contains("import vegetation_shaders::wesl::wind"));
		assert!(wood.contains("import vegetation_shaders::wesl::shading"));
		assert!(foliage.contains("import vegetation_shaders::wesl::curve"));
		assert!(foliage.contains("leaf_surface_vertex"));
		assert!(foliage.contains("grass_variation_palette"));
		assert!(foliage.contains("crown_layer_albedo"));
		assert!(select_wood.contains("import vegetation_shaders::wesl::lod"));
		assert!(select_foliage.contains("import vegetation_shaders::wesl::lod"));
		assert!(cull.contains("grove_lod_bucket"));
		assert!(prefix.contains("tiled_dispatch_groups"));
		assert!(!wood.contains("fn vertex(template:"), "WESL reserves `template` as a keyword");
		assert!(!cull.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(!prefix.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(!select_wood.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(!select_foliage.contains(" meta:"), "WESL reserves `meta` as a keyword");
		Ok(())
	}
}
