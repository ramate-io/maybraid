//! Inward gather, core flash, then time-staggered peel of additive energy shells.

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart};
use crate::layers::spirit_energy::{
	self, compile_crackle, compile_embers, compile_streaks, compile_wisps,
};
use crate::names::SPIRIT_SPAWN;

pub use spirit_energy::{SPIRIT_SPAWN_REVEAL, SPIRIT_SPAWN_SECS};

pub fn compile_gather_wisps(effects: &mut Assets<EffectAsset>, wisp: Handle<Image>) -> ParticlePart {
	compile_wisps(effects, wisp, SPIRIT_SPAWN, true)
}

pub fn compile_gather_crackle(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_crackle(effects, spark, SPIRIT_SPAWN, true)
}

pub fn compile_peel_wisps(effects: &mut Assets<EffectAsset>, wisp: Handle<Image>) -> ParticlePart {
	compile_wisps(effects, wisp, SPIRIT_SPAWN, false)
}

pub fn compile_peel_crackle(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_crackle(effects, spark, SPIRIT_SPAWN, false)
}

pub fn compile_streaks_layer(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_streaks(effects, spark, SPIRIT_SPAWN)
}

pub fn compile_embers_layer(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_embers(effects, spark, SPIRIT_SPAWN)
}

pub fn definition(
	gather_wisps: ParticlePart,
	gather_crackle: ParticlePart,
	peel_wisps: ParticlePart,
	peel_crackle: ParticlePart,
	streaks: ParticlePart,
	embers: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_SPAWN,
		[
			EffectLayer::particle(gather_wisps.clone()),
			EffectLayer::particle(gather_wisps).with_scale(1.18).with_delay(0.22),
			EffectLayer::particle(gather_crackle).with_delay(0.08),
			EffectLayer::light(spirit_energy::aura_light()).with_delay(0.12),
			EffectLayer::light(spirit_energy::core_flash()).with_delay(1.18),
			EffectLayer::particle(streaks.clone()).with_delay(1.16),
			EffectLayer::particle(embers).with_delay(1.2),
			EffectLayer::particle(streaks).with_scale(1.12).with_delay(1.42),
			EffectLayer::particle(peel_wisps.clone()).with_scale(0.85).with_delay(1.38),
			EffectLayer::particle(peel_crackle.clone()).with_delay(1.45),
			EffectLayer::particle(peel_wisps.clone()).with_scale(1.05).with_delay(1.95),
			EffectLayer::particle(peel_crackle).with_scale(1.15).with_delay(2.2),
			EffectLayer::particle(peel_wisps.clone()).with_scale(1.22).with_delay(2.5),
			EffectLayer::particle(peel_wisps).with_scale(1.4).with_delay(2.95),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_hanabi::EffectAsset;

	#[test]
	fn spawn_gathers_then_peels_for_about_four_seconds() {
		let mut effects = Assets::<EffectAsset>::default();
		let def = definition(
			compile_gather_wisps(&mut effects, Handle::default()),
			compile_gather_crackle(&mut effects, Handle::default()),
			compile_peel_wisps(&mut effects, Handle::default()),
			compile_peel_crackle(&mut effects, Handle::default()),
			compile_streaks_layer(&mut effects, Handle::default()),
			compile_embers_layer(&mut effects, Handle::default()),
		);
		assert_eq!(def.name, SPIRIT_SPAWN);
		assert!(!crate::layers::spirit::has_mesh_layer(&def));
		assert!(def.layers.len() >= 10);
		assert!(
			def.duration() >= 3.8 && def.duration() <= 4.7,
			"duration {}",
			def.duration()
		);
		assert!(
			(def.duration() - (2.95 + crate::layers::spirit_energy::WISP_LIFE_MAX)).abs() < 0.05
		);
		assert!(SPIRIT_SPAWN_REVEAL > 1.0 && SPIRIT_SPAWN_REVEAL < SPIRIT_SPAWN_SECS);
	}
}
