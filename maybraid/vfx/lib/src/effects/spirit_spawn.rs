//! Converging energy shells, core flash, then peeling translucent layers.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, ParticlePart};
use crate::layers::spirit_spawn;

#[derive(Component, Clone, Debug)]
pub struct SpiritSpawn {
	pub gather_wisps: ParticlePart,
	pub gather_crackle: ParticlePart,
	pub peel_wisps: ParticlePart,
	pub peel_crackle: ParticlePart,
	pub streaks: ParticlePart,
	pub embers: ParticlePart,
}

impl SpiritSpawn {
	pub fn new(
		gather_wisps: ParticlePart,
		gather_crackle: ParticlePart,
		peel_wisps: ParticlePart,
		peel_crackle: ParticlePart,
		streaks: ParticlePart,
		embers: ParticlePart,
	) -> Self {
		Self {
			gather_wisps,
			gather_crackle,
			peel_wisps,
			peel_crackle,
			streaks,
			embers,
		}
	}
}

impl EffectRecipe for SpiritSpawn {
	fn effect_definition(&self) -> EffectDefinition {
		spirit_spawn::definition(
			self.gather_wisps.clone(),
			self.gather_crackle.clone(),
			self.peel_wisps.clone(),
			self.peel_crackle.clone(),
			self.streaks.clone(),
			self.embers.clone(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::names::SPIRIT_SPAWN;

	fn stub_particle() -> ParticlePart {
		ParticlePart {
			name: SPIRIT_SPAWN.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: spirit_spawn::SPIRIT_SPAWN_SECS,
			shade: crate::composition::ParticleShade::Spirit,
		}
	}

	#[test]
	fn recipe_names_spawn_composite() {
		let particle = stub_particle();
		let recipe = SpiritSpawn::new(
			particle.clone(),
			particle.clone(),
			particle.clone(),
			particle.clone(),
			particle.clone(),
			particle,
		);
		assert_eq!(recipe.effect_definition().name, SPIRIT_SPAWN);
		assert!(recipe.effect_definition().layers.len() >= 10);
	}
}
