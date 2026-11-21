//! Importable WESL vegetation library plus CPU mirrors of the same contracts.
//!
//! Entry-point shaders (grass compute/vertex/fragment, and later bush wood/foliage)
//! should `import vegetation_shaders::wesl::…` rather than copy these functions.
//! Recipes and species appearance stay outside this math layer.
//!
//! ## Import-and-call example
//!
//! ```wesl
//! import vegetation_shaders::wesl::hash::{hash_u32, density_rank, plant_id};
//! import vegetation_shaders::wesl::curve::{
//!     blade_rest_sample, ribbon_vertex, stable_frame, attachment_frame,
//! };
//! import vegetation_shaders::wesl::wind::{wind_field, deform_along_curve};
//! import vegetation_shaders::wesl::lod::{projected_height_px, lod_bucket, nested_keep};
//! import vegetation_shaders::wesl::shading::{grass_albedo, grass_variation_palette, painterly_lit};
//! import vegetation_shaders::wesl::layout::{BladeDescriptor, GrassViewUniform};
//! ```
//!
//! [`import_smoke.wesl`](wesl/import_smoke.wesl) compiles this pattern.
//!
//! ## Bush follow-up ([#1638](https://github.com/ramate-io/maybraid/issues/1638))
//!
//! Reuse `curve` (including `stable_frame` / `attachment_frame`), `wind` (pass a
//! higher `stiffness` for wood), `shading`, `lod`, `hash`, and `layout`. Add a
//! branch-radius helper in the bush crate only if grass does not need it.
//! Keep wood/foliage entry points distinct from grass.

use bevy::prelude::*;
use bevy::shader::{load_shader_library, Shader};

pub mod curve;
pub mod hash;
pub mod layout;
pub mod lod;
pub mod shading;
pub mod wind;

pub use curve::{
	attachment_frame, blade_rest_sample, blade_width_at, ribbon_normal, ribbon_vertex,
	stable_frame, CurveSample, LocalFrame,
};
pub use hash::{
	density_rank, hash_lattice, hash_u32, hash_u32_to_unit, plant_id, value_noise_2, VegetationHash,
};
pub use layout::{
	BladeDescriptor, DrawIndexedIndirect, GeometryLod, GrassDrawParams, GrassPatchHeader,
	GrassViewUniform, BLADE_DESCRIPTOR_STRIDE, DRAW_INDEXED_INDIRECT_STRIDE, GRASS_LOD_CARD,
	GRASS_LOD_COUNT, GRASS_LOD_MID, GRASS_LOD_NEAR, GRASS_VIEW_KIND_CAMERA, GRASS_VIEW_KIND_SHADOW,
	PATCH_HEADER_STRIDE,
};
pub use lod::{
	lod_bucket, lod_fade, nested_keep, projected_height_px, LodDecision, LodThresholds,
	ProjectedSize,
};
pub use shading::{
	grass_albedo, grass_variation_palette, meadow_color_field, painterly_lit, two_sided_wrap,
	VegetationPalette,
};
pub use wind::{
	deform_along_curve, max_wind_displacement_m, wind_field, WindDeform, WindField, WindInputs,
};

/// Asset path for the import-smoke WESL (library composition check).
pub const IMPORT_SMOKE_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/wesl/import_smoke.wesl");

/// Registers reusable vegetation WESL modules as embedded shader assets.
pub struct VegetationWeslPlugin;

impl Plugin for VegetationWeslPlugin {
	fn build(&self, app: &mut App) {
		// Headless HCSG tests add this plugin through `VegetationViewPlugin` without
		// `RenderPlugin`. `load_shader_library!` allocates `Shader` handles immediately.
		// `init_asset` is not idempotent — calling it after `RenderPlugin` replaces
		// `Assets<Shader>` and panics on insert (`len is 26, index is 26`).
		if !app.world().contains_resource::<Assets<Shader>>() {
			app.init_asset::<Shader>();
		}
		load_shader_library!(app, "wesl/hash.wesl");
		load_shader_library!(app, "wesl/curve.wesl");
		load_shader_library!(app, "wesl/wind.wesl");
		load_shader_library!(app, "wesl/lod.wesl");
		load_shader_library!(app, "wesl/shading.wesl");
		load_shader_library!(app, "wesl/layout.wesl");
		load_shader_library!(app, "wesl/import_smoke.wesl");
	}
}

impl VegetationWeslPlugin {
	/// Import path Bevy assigns to `embedded://vegetation_shaders/wesl/<stem>.wesl`.
	pub fn import_path(stem: &str) -> String {
		format!("vegetation_shaders::wesl::{stem}")
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use bevy::shader::Shader;

	use super::*;

	#[test]
	fn import_paths_match_embedded_wesl_layout() -> anyhow::Result<()> {
		assert_eq!(VegetationWeslPlugin::import_path("hash"), "vegetation_shaders::wesl::hash");
		assert_eq!(VegetationWeslPlugin::import_path("curve"), "vegetation_shaders::wesl::curve");
		assert_eq!(IMPORT_SMOKE_SHADER, "embedded://vegetation_shaders/wesl/import_smoke.wesl");
		Ok(())
	}

	#[test]
	fn embedded_wesl_library_registers_with_asset_plugin() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		app.add_plugins(VegetationWeslPlugin);
		app.update();
		assert!(app.world().get_resource::<Assets<Shader>>().is_some());
	}

	#[test]
	fn does_not_reinitialize_shader_assets_after_render_like_hosts() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(AssetPlugin::default());
		app.init_asset::<Shader>();
		app.add_plugins(VegetationWeslPlugin);
		app.update();
		assert!(app.world().get_resource::<Assets<Shader>>().is_some());
	}

	#[test]
	fn import_smoke_source_imports_each_library_module() -> anyhow::Result<()> {
		let smoke = include_str!("wesl/import_smoke.wesl");
		for stem in ["hash", "curve", "wind", "lod", "shading", "layout"] {
			assert!(
				smoke.contains(&format!("import vegetation_shaders::wesl::{stem}")),
				"import_smoke.wesl must import {stem}"
			);
		}
		let _modules = [
			include_str!("wesl/hash.wesl"),
			include_str!("wesl/curve.wesl"),
			include_str!("wesl/wind.wesl"),
			include_str!("wesl/lod.wesl"),
			include_str!("wesl/shading.wesl"),
			include_str!("wesl/layout.wesl"),
		];
		Ok(())
	}
}
