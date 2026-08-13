//! Chico vegetation shaders: reusable Bevy [`Material`] types with embedded WGSL for bark sticks and stylized canopy foliage.
//!
//! - [`ChicoStickMaterial`] — edge-accent PBR (from `playgrounds/objects/assets/shaders/edge_material.wgsl`).
//! - [`ChicoLeafMaterial`] — object-space leafy breakup + vertex sway + double-sided PBR.

use bevy::prelude::*;

mod chico_leaf_material;
mod chico_stick_material;

pub use chico_leaf_material::{ChicoLeafMaterial, ChicoLeafMaterialPlugin};
pub use chico_stick_material::{ChicoStickMaterial, ChicoStickMaterialPlugin};

/// Convenience plugin that registers both vegetation materials.
pub struct ChicoVegetationShadersPlugin;

impl Plugin for ChicoVegetationShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ChicoStickMaterialPlugin, ChicoLeafMaterialPlugin));
	}
}
