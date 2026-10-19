//! Epiphant / Bikanwarda **placeholder** tattoo UV sources ([#1128](https://github.com/ramate-io/maybraid/issues/1128)).
//!
//! Final authored `TEXCOORD_0` should switch every slot to
//! [`SkinTattooUvSource::MeshUv0`]. Requirements are documented in
//! `maybraid/assets/characters/tattoos/README.md`.

use character_shaders::SkinTattooUvSource;

use crate::assembly::CharacterPartSlot;

/// Provisional unwrap per part until Liam exports usable UV0 on each GLB.
pub fn epiphant_placeholder_uv_source(slot: CharacterPartSlot) -> SkinTattooUvSource {
	match slot {
		CharacterPartSlot::BodyMesh => SkinTattooUvSource::ProvisionalBodyCylindricalZ,
		CharacterPartSlot::HeadMesh | CharacterPartSlot::HeadRig => {
			SkinTattooUvSource::ProvisionalHeadCylindricalY
		}
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
			SkinTattooUvSource::ProvisionalEarPlanarXy
		}
		CharacterPartSlot::Nose => SkinTattooUvSource::ProvisionalTrunkCylindricalY,
		_ => SkinTattooUvSource::MeshUv0,
	}
}
