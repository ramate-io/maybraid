//! Impact ooze: droplets plus delayed ground drips.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, ParticlePart};
use crate::layers::ooze;

#[derive(Component, Clone, Debug)]
pub struct OozeBurst {
	pub droplets: ParticlePart,
	pub drips: ParticlePart,
}

impl OozeBurst {
	pub fn new(droplets: ParticlePart, drips: ParticlePart) -> Self {
		Self { droplets, drips }
	}
}

impl EffectRecipe for OozeBurst {
	fn effect_definition(&self) -> EffectDefinition {
		ooze::definition(self.droplets.clone(), self.drips.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::ParticleShade;
	use crate::names::OOZE_BURST;

	fn stub_particle(life: f32) -> ParticlePart {
		ParticlePart {
			name: "stub".into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: life,
			shade: ParticleShade::Ooze,
		}
	}

	#[test]
	fn recipe_names_the_composite() {
		let recipe = OozeBurst::new(stub_particle(0.5), stub_particle(1.0));
		assert_eq!(recipe.effect_definition().name, OOZE_BURST);
		assert_eq!(recipe.effect_definition().layers.len(), 2);
	}
}
