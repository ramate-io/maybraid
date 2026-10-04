//! Brief emissive burst shape plus an optional light pulse.

use bevy::prelude::*;

use crate::composition::{
	EffectDefinition, EffectLayer, LightPulse, LobeKind, LobeSpec, MeshPart, FLASH,
};

pub const FLASH_FADE: f32 = 0.12;
pub const FLASH_PEAK: f32 = 40_000.0;
pub const FLASH_RANGE: f32 = 6.0;

pub fn pulse() -> LightPulse {
	LightPulse {
		color: Color::srgb(1.0, 0.82, 0.45),
		peak_intensity: FLASH_PEAK,
		range: FLASH_RANGE,
		fade: FLASH_FADE,
	}
}

pub fn compile_mesh(mesh: Handle<Mesh>) -> MeshPart {
	MeshPart::duration_from_lobes(
		FLASH,
		mesh,
		LobeKind::Flash,
		vec![
			LobeSpec {
				offset: Vec3::ZERO,
				scale: Vec3::new(0.28, 0.18, 0.22),
				euler: Vec3::new(0.25, 0.6, -0.15),
				expand: 1.55,
				rise: 0.0,
				roll: 0.2,
				duration: FLASH_FADE,
			},
			LobeSpec {
				offset: Vec3::new(0.05, 0.03, -0.03),
				scale: Vec3::new(0.14, 0.08, 0.11),
				euler: Vec3::new(-0.7, 0.3, 0.5),
				expand: 1.1,
				rise: 0.0,
				roll: 0.4,
				duration: FLASH_FADE * 0.7,
			},
		],
	)
}

pub fn definition(mesh: MeshPart) -> EffectDefinition {
	EffectDefinition::new(FLASH, [EffectLayer::mesh(mesh), EffectLayer::light(pulse())])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flash_owns_shape_and_light() {
		let def = definition(compile_mesh(Handle::default()));
		assert_eq!(def.layers.len(), 2);
		assert!((def.duration() - FLASH_FADE).abs() < 1e-4);
	}
}
