//! Clothing shaders: named [`MaterialRef`] recipes with a tiny hem sway.

use bevy::prelude::*;

mod clothing_material;

pub use clothing_material::{
	ClothingMaterialUniform, ClothingShaderKind, ClothingShaderMaterial,
	ClothingShaderMaterialPlugin, KIND_CLOTH, KIND_GLITTER, KIND_HAWAIIAN, KIND_SCALES,
	KIND_SPACE_SUIT, KIND_TATTERED, KIND_WIZARDS_VEINS,
};

/// Registers clothing materials used by Crozon [`material_ref::MaterialLib`]s.
pub struct CrozonCharacterShadersPlugin;

impl Plugin for CrozonCharacterShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ClothingShaderMaterialPlugin);
	}
}
