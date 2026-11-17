//! Outward spiritual dispersal: staggered wispy shells, crackle, streaks, core flash.

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectLayer, EffectPart, ParticlePart};
use crate::layers::spirit_energy::{
	self, compile_crackle, compile_embers, compile_streaks, compile_wisps,
};
use crate::names::SPIRIT_DISSOLVE;

pub use spirit_energy::SPIRIT_DISSOLVE_SECS;

pub fn compile_wisps_layer(effects: &mut Assets<EffectAsset>, wisp: Handle<Image>) -> ParticlePart {
	compile_wisps(effects, wisp, SPIRIT_DISSOLVE, false)
}

pub fn compile_crackle_layer(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_crackle(effects, spark, SPIRIT_DISSOLVE, false)
}

pub fn compile_streaks_layer(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_streaks(effects, spark, SPIRIT_DISSOLVE)
}

pub fn compile_embers_layer(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_embers(effects, spark, SPIRIT_DISSOLVE)
}

pub fn definition(
	wisps: ParticlePart,
	crackle: ParticlePart,
	streaks: ParticlePart,
	embers: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_DISSOLVE,
		[
			EffectLayer::light(spirit_energy::core_flash()),
			EffectLayer::particle(streaks),
			EffectLayer::particle(embers).with_delay(0.04),
			EffectLayer::particle(wisps.clone()).with_scale(0.82),
			EffectLayer::particle(crackle.clone()).with_delay(0.08),
			EffectLayer::light(spirit_energy::aura_light()).with_delay(0.05),
			EffectLayer::particle(wisps.clone()).with_scale(1.02).with_delay(0.55),
			EffectLayer::particle(crackle).with_scale(1.12).with_delay(0.7),
			EffectLayer::particle(wisps.clone()).with_scale(1.22).with_delay(1.15),
			EffectLayer::particle(wisps).with_scale(1.38).with_delay(1.55),
		],
	)
}

pub fn has_mesh_layer(definition: &EffectDefinition) -> bool {
	definition
		.layers
		.iter()
		.any(|layer| matches!(layer.part, EffectPart::Mesh(_)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_hanabi::EffectAsset;

	#[test]
	fn dissolve_is_a_staggered_particle_burst() {
		let mut effects = Assets::<EffectAsset>::default();
		let def = definition(
			compile_wisps_layer(&mut effects, Handle::default()),
			compile_crackle_layer(&mut effects, Handle::default()),
			compile_streaks_layer(&mut effects, Handle::default()),
			compile_embers_layer(&mut effects, Handle::default()),
		);
		assert_eq!(def.name, SPIRIT_DISSOLVE);
		assert!(!has_mesh_layer(&def));
		assert!(def.layers.len() >= 8);
		assert!(
			def.duration() >= 2.5 && def.duration() <= 4.0,
			"duration {}",
			def.duration()
		);
		assert!((def.duration() - (1.55 + crate::layers::spirit_energy::WISP_LIFE_MAX)).abs() < 0.05);
	}
}
