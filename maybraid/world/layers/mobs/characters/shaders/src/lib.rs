//! character shaders: clothing looks, face idle (blink / mouth), and skin tattoos.

use bevy::prelude::*;

mod clothing_material;
mod face_material;
mod skin_material;
mod skin_tattoo_uv;

pub use clothing_material::{
	ClothingMaterialUniform, ClothingShaderKind, ClothingShaderMaterial,
	ClothingShaderMaterialPlugin, FLAG_NO_SWAY, KIND_BRUSHED_METAL, KIND_CLOTH, KIND_GLITTER,
	KIND_HAWAIIAN, KIND_LAVA_VEINS, KIND_SCALES, KIND_SPACE_SUIT, KIND_TATTERED,
	KIND_WIZARDS_VEINS,
};
pub use face_material::{
	blink_envelope, eye_palette, lid_wrap, lip_opening, mouth_deform, mouth_open_envelope,
	mouth_open_envelope_at, mouth_open_rate, mouth_palette, FaceMaterialUniform, FaceShaderKind,
	FaceShaderMaterial, FaceShaderMaterialPlugin, DEFAULT_MOUTH_OPEN_RATE, EYE_PALETTE_HIGHLIGHT,
	EYE_PALETTE_IRIS, EYE_PALETTE_IRIS_SECONDARY, EYE_PALETTE_LID, EYE_PALETTE_PUPIL,
	EYE_PALETTE_SCLERA, EYE_SCALAR_PUPIL_SHAPE, KIND_EYE, KIND_MOUTH, MOUTH_PALETTE_CREASE,
	MOUTH_PALETTE_HIGHLIGHT, MOUTH_PALETTE_INTERIOR, MOUTH_PALETTE_LIP, MOUTH_PALETTE_TEETH,
	MOUTH_SCALAR_OPEN_RATE, PUPIL_SHAPE_ROUND, PUPIL_SHAPE_SLIT, RECIPE_FACE_EYE,
	RECIPE_FACE_MOUTH,
};
pub use skin_material::{
	SkinMaterialUniform, SkinShaderMaterial, SkinShaderMaterialPlugin, SkinTattoo, SkinTattooMask,
	SkinTattooMasks, DEFAULT_TATTOO_GLOW, DEFAULT_TATTOO_INTENSITY, DEFAULT_TATTOO_PULSE_AMP,
	DEFAULT_TATTOO_PULSE_RATE, FLAG_UV_DEBUG, KIND_BODY, KIND_EAR, KIND_HEAD, KIND_TRUNK,
	RECIPE_SKIN_TATTOO_BODY, RECIPE_SKIN_TATTOO_EAR, RECIPE_SKIN_TATTOO_HEAD,
	RECIPE_SKIN_TATTOO_TRUNK, SKIN_PALETTE_BASE, SKIN_PALETTE_GLOW, SKIN_SCALAR_INTENSITY,
	SKIN_SCALAR_PULSE_AMP, SKIN_SCALAR_PULSE_RATE,
};
pub use skin_tattoo_uv::{
	body_provisional_uv, ear_provisional_uv, head_provisional_uv, trunk_provisional_uv,
	SkinTattooUvSource, SKIN_SCALAR_UV_SOURCE,
};

/// Registers clothing, face, and skin-tattoo materials used by character [`material_ref::MaterialLib`]s.
pub struct CharacterShadersPlugin;

impl Plugin for CharacterShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			ClothingShaderMaterialPlugin,
			FaceShaderMaterialPlugin,
			SkinShaderMaterialPlugin,
		));
	}
}
