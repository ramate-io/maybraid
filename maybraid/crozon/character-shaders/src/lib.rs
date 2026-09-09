//! Crozon character shaders: clothing looks and face idle (blink / mouth).

use bevy::prelude::*;

mod clothing_material;
mod face_material;

pub use clothing_material::{
	ClothingMaterialUniform, ClothingShaderKind, ClothingShaderMaterial,
	ClothingShaderMaterialPlugin, FLAG_NO_SWAY, KIND_BRUSHED_METAL, KIND_CLOTH, KIND_GLITTER,
	KIND_HAWAIIAN, KIND_LAVA_VEINS, KIND_SCALES, KIND_SPACE_SUIT, KIND_TATTERED,
	KIND_WIZARDS_VEINS,
};
pub use face_material::{
	blink_envelope, FaceMaterialUniform, FaceShaderKind, FaceShaderMaterial,
	FaceShaderMaterialPlugin, KIND_EYE, KIND_MOUTH, RECIPE_FACE_EYE, RECIPE_FACE_MOUTH,
};

/// Registers clothing and face materials used by Crozon [`material_ref::MaterialLib`]s.
pub struct CrozonCharacterShadersPlugin;

impl Plugin for CrozonCharacterShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ClothingShaderMaterialPlugin, FaceShaderMaterialPlugin));
	}
}
