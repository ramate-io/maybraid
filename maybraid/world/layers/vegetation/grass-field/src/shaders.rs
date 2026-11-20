//! Grass compute / draw entry points. They import `vegetation_shaders::wesl::*`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;

pub const SELECT_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/select.wesl");
pub const FINALIZE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/finalize.wesl");
pub const BLADE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/shaders/blade.wesl");

pub struct GrassShadersPlugin;

impl Plugin for GrassShadersPlugin {
	fn build(&self, app: &mut App) {
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
		assert!(select.contains("import vegetation_shaders::wesl::lod"));
		assert!(select.contains("import vegetation_shaders::wesl::layout"));
		assert!(select.contains("import vegetation_shaders::wesl::parting"));
		assert!(blade.contains("import vegetation_shaders::wesl::curve"));
		assert!(blade.contains("import vegetation_shaders::wesl::wind"));
		assert!(blade.contains("import vegetation_shaders::wesl::shading"));
		assert!(blade.contains("import vegetation_shaders::wesl::parting"));
		assert!(!blade.contains("fn vertex(template:"), "WESL reserves `template` as a keyword");
		assert!(finalize.contains("DrawIndexedIndirect"));
		Ok(())
	}
}
