//! Corpse dissolution into staggered additive energy shells.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, ParticlePart};
use crate::layers::spirit;

#[derive(Component, Clone, Debug)]
pub struct SpiritDissolve {
	pub wisps: ParticlePart,
	pub crackle: ParticlePart,
	pub streaks: ParticlePart,
	pub embers: ParticlePart,
}

impl SpiritDissolve {
	pub fn new(
		wisps: ParticlePart,
		crackle: ParticlePart,
		streaks: ParticlePart,
		embers: ParticlePart,
	) -> Self {
		Self { wisps, crackle, streaks, embers }
	}
}

impl EffectRecipe for SpiritDissolve {
	fn effect_definition(&self) -> EffectDefinition {
		spirit::definition(
			self.wisps.clone(),
			self.crackle.clone(),
			self.streaks.clone(),
			self.embers.clone(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::names::SPIRIT_DISSOLVE;

	#[test]
	fn recipe_matches_standalone_layer() {
		let particle = ParticlePart {
			name: SPIRIT_DISSOLVE.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: 1.0,
			shade: crate::composition::ParticleShade::Spirit,
		};
		let recipe = SpiritDissolve::new(particle.clone(), particle.clone(), particle.clone(), particle);
		assert_eq!(recipe.effect_definition().name, SPIRIT_DISSOLVE);
	}
}
