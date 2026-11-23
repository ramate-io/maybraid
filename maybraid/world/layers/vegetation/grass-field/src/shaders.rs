//! Grass compute / draw entry points. They import `vegetation_shaders::wesl::*`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;

pub const CULL_GROVES_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/cull_groves.wesl");
pub const PREFIX_SPANS_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/prefix_spans.wesl");
pub const SELECT_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/select.wesl");
pub const FINALIZE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/finalize.wesl");
pub const BLADE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/blade.wesl");

pub struct GrassShadersPlugin;

impl Plugin for GrassShadersPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "shaders/cull_groves.wesl");
		embedded_asset!(app, "shaders/prefix_spans.wesl");
		embedded_asset!(app, "shaders/select.wesl");
		embedded_asset!(app, "shaders/finalize.wesl");
		embedded_asset!(app, "shaders/blade.wesl");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shader_refs_match_embedded_layout() -> anyhow::Result<()> {
		assert_eq!(SELECT_SHADER, "embedded://vegetation_grass_field/shaders/select.wesl");
		assert_eq!(BLADE_SHADER, "embedded://vegetation_grass_field/shaders/blade.wesl");
		Ok(())
	}

	#[test]
	fn embedded_entry_points_register() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		app.add_plugins(GrassShadersPlugin);
		app.update();
	}

	#[test]
	fn entry_points_import_wesl_library() -> anyhow::Result<()> {
		let select = include_str!("shaders/select.wesl");
		let blade = include_str!("shaders/blade.wesl");
		let finalize = include_str!("shaders/finalize.wesl");
		let cull = include_str!("shaders/cull_groves.wesl");
		let prefix = include_str!("shaders/prefix_spans.wesl");
		assert!(select.contains("import vegetation_shaders::wesl::lod"));
		assert!(select.contains("import vegetation_shaders::wesl::layout"));
		assert!(cull.contains("grove_lod_bucket"));
		assert!(prefix.contains("tiled_dispatch_groups"));
		assert!(blade.contains("import vegetation_shaders::wesl::curve"));
		assert!(blade.contains("import vegetation_shaders::wesl::wind"));
		assert!(blade.contains("import vegetation_shaders::wesl::shading"));
		assert!(!blade.contains("fn vertex(template:"), "WESL reserves `template` as a keyword");
		assert!(!cull.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(!prefix.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(!select.contains(" meta:"), "WESL reserves `meta` as a keyword");
		assert!(finalize.contains("DrawIndexedIndirect"));
		Ok(())
	}
}
