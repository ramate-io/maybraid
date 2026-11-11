//! Inward spiritual materialization: converging shell, streaks, and core reveal flash.

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectLayer, MeshPart, ParticlePart};
use crate::layers::spirit;
use crate::layers::spirit_energy::{
	self, compile_embers, compile_inward_streaks, compile_shell_motes, MOTE_LIFE_MAX, SHELL_FADE,
};
use crate::names::SPIRIT_SPAWN;
use crate::parts::particle::assets::FlipbookAsset;

pub const SPIRIT_SPAWN_SECS: f32 = 1.65;
pub const SPIRIT_SPAWN_REVEAL: f32 = 0.82;

pub fn compile_shell(mesh: Handle<Mesh>) -> MeshPart {
	spirit_energy::compile_shell(mesh, SPIRIT_SPAWN, SHELL_FADE)
}

pub fn compile_streaks(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	compile_inward_streaks(effects, spark, SPIRIT_SPAWN)
}

pub fn compile_motes(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
) -> ParticlePart {
	compile_shell_motes(effects, smoke, SPIRIT_SPAWN, false)
}

pub fn compile_embers_layer(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
) -> ParticlePart {
	compile_embers(effects, spark, SPIRIT_SPAWN)
}

pub fn definition(
	shell: MeshPart,
	streaks: ParticlePart,
	motes: ParticlePart,
	embers: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_SPAWN,
		[
			EffectLayer::particle(streaks),
			EffectLayer::mesh(shell),
			EffectLayer::particle(motes),
			EffectLayer::light(spirit::aura_light()).with_delay(0.08),
			EffectLayer::light(spirit::core_flash()).with_delay(0.52),
			EffectLayer::particle(embers).with_delay(0.45),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_hanabi::EffectAsset;

	#[test]
	fn spawn_converges_faster_than_dissolve() {
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
		assert_eq!(def.name, SPIRIT_SPAWN);
		assert!(def.duration() <= spirit::SPIRIT_DISSOLVE_SECS + 0.25);
		let _ = MOTE_LIFE_MAX;
	}
}
