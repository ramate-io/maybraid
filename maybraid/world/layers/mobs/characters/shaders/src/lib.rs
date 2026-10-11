//! character shaders: clothing looks and face idle (blink / mouth).

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
	blink_envelope, eye_glow_intensity, eye_palette, lid_wrap, lip_opening, mouth_deform,
	mouth_open_envelope, mouth_open_envelope_at, mouth_open_rate, mouth_palette,
	set_eye_glow_uniform, FaceMaterialUniform, FaceShaderKind, FaceShaderMaterial,
	FaceShaderMaterialPlugin, DEFAULT_EYE_GLOW_EMISSIVE_GAIN, DEFAULT_MOUTH_OPEN_RATE,
	EYE_PALETTE_HIGHLIGHT, EYE_PALETTE_IRIS, EYE_PALETTE_IRIS_SECONDARY, EYE_PALETTE_LID,
	EYE_PALETTE_PUPIL, EYE_PALETTE_SCLERA, EYE_SCALAR_GLOW_COLOR_VEC,
	EYE_SCALAR_GLOW_INTENSITY, EYE_SCALAR_GLOW_INTENSITY_COMPONENT, EYE_SCALAR_PUPIL_SHAPE,
	GOD_BEAM_EYE_GLOW_COLOR, GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN, GOD_BEAM_EYE_GLOW_INTENSITY,
	KIND_EYE, KIND_MOUTH, MOUTH_PALETTE_CREASE, MOUTH_PALETTE_HIGHLIGHT, MOUTH_PALETTE_INTERIOR,
	MOUTH_PALETTE_LIP, MOUTH_PALETTE_TEETH, MOUTH_SCALAR_OPEN_RATE, PUPIL_SHAPE_ROUND,
	PUPIL_SHAPE_SLIT, RECIPE_FACE_EYE, RECIPE_FACE_MOUTH,
};

/// Registers clothing and face materials used by character [`material_ref::MaterialLib`]s.
pub struct CharacterShadersPlugin;

impl Plugin for CharacterShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ClothingShaderMaterialPlugin, FaceShaderMaterialPlugin));
	}
}
