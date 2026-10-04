//! Reusable flash and optional light pulse.

use crate::composition::{EffectDefinition, EffectLayer, LightPulse, FLASH};

pub const FLASH_FADE: f32 = 0.1;
pub const FLASH_PEAK: f32 = 1_200_000.0;
pub const FLASH_RANGE: f32 = 6.0;

pub fn pulse() -> LightPulse {
	LightPulse {
		color: bevy::prelude::Color::srgb(1.0, 0.82, 0.45),
		peak_intensity: FLASH_PEAK,
		range: FLASH_RANGE,
		fade: FLASH_FADE,
	}
}

pub fn definition() -> EffectDefinition {
	EffectDefinition::new(FLASH, [EffectLayer::light(pulse())])
}
