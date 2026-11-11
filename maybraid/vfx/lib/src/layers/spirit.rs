//! Outward spiritual burst: shell, streaks, motes, embers, and core flash.

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectLayer, LightPulse, MeshPart, ParticlePart};
use crate::layers::spirit_energy::{
	self, compile_embers, compile_outward_streaks, compile_shell_motes,
	BURST_LIGHT_FADE, BURST_LIGHT_PEAK, BURST_LIGHT_RANGE, CORE_FLASH_FADE, CORE_FLASH_PEAK,
	CORE_FLASH_RANGE, MOTE_LIFE_MAX, SHELL_FADE, STREAK_LIFE_MAX,
};
use crate::names::SPIRIT_DISSOLVE;
use crate::palette::ExplosionPalette;
use crate::parts::particle::assets::FlipbookAsset;

pub const SPIRIT_DISSOLVE_SECS: f32 = 2.05;

pub fn core_flash() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(LinearRgba::rgb(1.35, 1.32, 1.42)),
		peak_intensity: CORE_FLASH_PEAK,
		range: CORE_FLASH_RANGE,
		fade: CORE_FLASH_FADE,
	}
}

pub fn aura_light() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(LinearRgba::rgb(0.65, 0.95, 1.25)),
		peak_intensity: BURST_LIGHT_PEAK,
		range: BURST_LIGHT_RANGE,
		fade: BURST_LIGHT_FADE,
	}
}

pub fn compile_shell(mesh: Handle<Mesh>) -> MeshPart {
	spirit_energy::compile_shell(mesh, SPIRIT_DISSOLVE, SHELL_FADE)
}

pub fn compile_streaks(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_outward_streaks(effects, spark, SPIRIT_DISSOLVE)
}

pub fn compile_motes(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
) -> ParticlePart {
	compile_shell_motes(effects, smoke, SPIRIT_DISSOLVE, true)
}

pub fn compile_embers_layer(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
) -> ParticlePart {
	compile_embers(effects, spark, SPIRIT_DISSOLVE)
}

pub fn definition(
	shell: MeshPart,
	streaks: ParticlePart,
	motes: ParticlePart,
	embers: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_DISSOLVE,
		[
			EffectLayer::light(core_flash()),
			EffectLayer::mesh(shell),
			EffectLayer::particle(streaks),
			EffectLayer::particle(motes).with_delay(0.03),
			EffectLayer::particle(embers).with_delay(0.06),
			EffectLayer::light(aura_light()).with_delay(0.02),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_hanabi::EffectAsset;

	#[test]
	fn dissolve_is_an_outward_burst() {
		let mut effects = Assets::<EffectAsset>::default();
		let smoke = FlipbookAsset {
			image: Handle::default(),
			grid: UVec2::ONE,
			frame_count: 1,
			playback: 1.0,
		};
		let def = definition(
			compile_shell(Handle::default()),
			compile_streaks(&mut effects, Handle::default()),
			compile_motes(&mut effects, &smoke),
			compile_embers_layer(&mut effects, Handle::default()),
		);
		assert_eq!(def.name, SPIRIT_DISSOLVE);
		assert!(def.layers.len() >= 5);
		assert!(
			(def.duration() - MOTE_LIFE_MAX - 0.06).abs() < 0.2,
			"duration {}",
			def.duration()
		);
		let _ = STREAK_LIFE_MAX;
		let _ = spirit_energy::SHELL_FADE;
	}
}
