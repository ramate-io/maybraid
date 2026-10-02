//! Vegetation shaders: reusable Bevy [`Material`] types with embedded WGSL.
//!
//! - [`StickMaterial`] — edge-accent PBR (from `playgrounds/objects/assets/shaders/edge_material.wgsl`).
//! - [`LeafMaterial`] — object-space leafy breakup + vertex sway + split light.
//!   Noisy rim `discard` at every distance. Interior holes near/mid only.
//!   Fake canopy occlusion. Opaque.
//! - [`FrondMaterial`] — palette + tip-weighted sway + double-sided PBR. Opaque;
//!   no cheese / `discard` (authored frond kit silhouette).
//! - [`BumpOutMaterial`] — terrain-mesh displacement + neighborhood rasters + fragment cheese.
//!
//! [`VegetationMaterialLib`] claims leaf / stick / frond recipes only. Compose it with
//! other domain libs (bump-out, Standard) in the app crate.

use bevy::prelude::*;

mod bump_out_material;
mod frond_material;
mod leaf_material;
mod stick_material;
mod material_lib;

pub use bump_out_material::{
	BumpOutMaterial, BumpOutMaterialPlugin, BumpOutUniform, BUMP_OUT_MATERIAL,
	RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION, RASTER_DENSITY,
	RASTER_HEIGHT_DEVIATION,
};
pub use frond_material::{FrondMaterial, FrondMaterialPlugin};
pub use leaf_material::{LeafMaterial, LeafMaterialPlugin};
pub use stick_material::{StickMaterial, StickMaterialPlugin};
pub use material_lib::{
	init_vegetation_material_caches, FrondMaterialRefCache, LeafMaterialRefCache,
	VegetationMaterialLib, VegetationMaterialRefPlugin, StandaloneVegetationMaterialLib,
	StickMaterialRefCache,
};

/// Convenience plugin that registers vegetation materials.
pub struct VegetationShadersPlugin;

impl Plugin for VegetationShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			StickMaterialPlugin,
			LeafMaterialPlugin,
			FrondMaterialPlugin,
		));
		if !app.is_plugin_added::<BumpOutMaterialPlugin>() {
			app.add_plugins(BumpOutMaterialPlugin);
		}
	}
}
