//! Tattoo mask UV selection for [`SkinShaderMaterial`].
//!
//! The shader samples `TEXCOORD_0` when [`SkinTattooUvSource::MeshUv0`] is set and the mesh
//! provides UVs. Provisional bind-pose unwraps are explicit per [`MaterialRef`] — content
//! recipes (for example Epiphant placeholders) choose them; the shader does not infer unwrap
//! from anatomy / mask kind alone.

use bevy::prelude::Vec3;

/// How a skin-tattoo material maps fragment position to mask UV.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SkinTattooUvSource {
	/// Sample mesh `TEXCOORD_0` when present.
	#[default]
	MeshUv0 = 0,
	/// Cylindrical about +Z (`epiphant.glb` bind-pose AABB).
	ProvisionalBodyCylindricalZ = 1,
	/// Planar XY (`epiphant_ear_left.glb` bind-pose AABB).
	ProvisionalEarPlanarXy = 2,
	/// Cylindrical about +Y (`trunkish_nose.glb` bind-pose AABB).
	ProvisionalTrunkCylindricalY = 3,
	/// Cylindrical about +Y (`meerkat_head_pronograde.glb` bind-pose AABB).
	///
	/// Epiphant placeholder only — authored head UV0 is a collapsed pole until GLB
	/// re-unwrap (see `maybraid/assets/characters/tattoos/README.md`).
	ProvisionalHeadCylindricalY = 4,
}

impl SkinTattooUvSource {
	pub const fn as_u32(self) -> u32 {
		self as u32
	}

	pub fn from_u32(value: u32) -> Self {
		match value {
			1 => Self::ProvisionalBodyCylindricalZ,
			2 => Self::ProvisionalEarPlanarXy,
			3 => Self::ProvisionalTrunkCylindricalY,
			4 => Self::ProvisionalHeadCylindricalY,
			_ => Self::MeshUv0,
		}
	}

	/// Packed into [`SKIN_SCALAR_UV_SOURCE`] on the material ref.
	pub const fn as_scalar(self) -> f32 {
		self.as_u32() as f32
	}
}

pub const SKIN_SCALAR_UV_SOURCE: usize = 3;

/// WGSL / Rust parity for provisional body unwrap (not selected unless requested).
pub fn body_provisional_uv(local: Vec3) -> bevy::prelude::Vec2 {
	const Z_MIN: f32 = -0.9804133;
	const Z_SPAN: f32 = 2.0035005;
	let u = local.x.atan2(local.y) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.z - Z_MIN) / Z_SPAN).clamp(0.0, 1.0);
	bevy::prelude::Vec2::new(u, v)
}

pub fn ear_provisional_uv(local: Vec3) -> bevy::prelude::Vec2 {
	const X_MIN: f32 = -0.295_222_76;
	const X_SPAN: f32 = 2.330_160_9;
	const Y_MIN: f32 = -1.652_735_1;
	const Y_SPAN: f32 = 2.981_944_3;
	let u = ((local.x - X_MIN) / X_SPAN).clamp(0.0, 1.0);
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	bevy::prelude::Vec2::new(u, v)
}

pub fn trunk_provisional_uv(local: Vec3) -> bevy::prelude::Vec2 {
	const Y_MIN: f32 = -1.932_509;
	const Y_SPAN: f32 = 2.469_754_7;
	let u = local.x.atan2(local.z) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	bevy::prelude::Vec2::new(u, v)
}

pub fn head_provisional_uv(local: Vec3) -> bevy::prelude::Vec2 {
	const Y_MIN: f32 = -0.979_147_3;
	const Y_SPAN: f32 = 2.041_864_7;
	let u = local.x.atan2(local.z) * std::f32::consts::FRAC_1_PI * 0.5 + 0.5;
	let v = ((local.y - Y_MIN) / Y_SPAN).clamp(0.0, 1.0);
	bevy::prelude::Vec2::new(u, v)
}
