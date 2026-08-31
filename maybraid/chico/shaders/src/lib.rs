//! Chico vegetation shaders: reusable Bevy [`Material`] types with embedded WGSL.
//!
//! - [`ChicoStickMaterial`] — edge-accent PBR (from `playgrounds/objects/assets/shaders/edge_material.wgsl`).
//! - [`ChicoLeafMaterial`] — object-space leafy breakup + vertex sway + split light.
//!   Noisy rim `discard` at every distance. Interior holes near/mid only.
//!   Fake canopy occlusion. Opaque.
//! - [`ChicoFrondMaterial`] — palette + tip-weighted sway + double-sided PBR. Opaque;
//!   no cheese / `discard` (authored frond kit silhouette).
//! - [`BumpOutMaterial`] — terrain-mesh displacement + neighborhood rasters + fragment cheese.
//!
//! [`ChicoMaterialLib`] claims leaf / stick / frond recipes only. Compose it with
//! other domain libs (bump-out, Standard) in the app crate.

use bevy::prelude::*;

mod chico_bump_out_material;
mod chico_frond_material;
mod chico_leaf_material;
mod chico_stick_material;
mod material_lib;

pub use chico_bump_out_material::{
	BumpOutMaterial, BumpOutMaterialPlugin, BumpOutUniform, CHICO_BUMP_OUT_MATERIAL,
	RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION, RASTER_DENSITY,
	RASTER_HEIGHT_DEVIATION,
};
pub use chico_frond_material::{ChicoFrondMaterial, ChicoFrondMaterialPlugin};
pub use chico_leaf_material::{ChicoLeafMaterial, ChicoLeafMaterialPlugin};
pub use chico_stick_material::{ChicoStickMaterial, ChicoStickMaterialPlugin};
pub use material_lib::{
	init_chico_material_caches, ChicoFrondMaterialRefCache, ChicoLeafMaterialRefCache,
	ChicoMaterialLib, ChicoMaterialRefPlugin, ChicoStandaloneMaterialLib,
	ChicoStickMaterialRefCache,
};

/// Convenience plugin that registers vegetation materials.
pub struct ChicoVegetationShadersPlugin;

impl Plugin for ChicoVegetationShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			ChicoStickMaterialPlugin,
			ChicoLeafMaterialPlugin,
			ChicoFrondMaterialPlugin,
		));
		if !app.is_plugin_added::<BumpOutMaterialPlugin>() {
			app.add_plugins(BumpOutMaterialPlugin);
		}
	}
}
