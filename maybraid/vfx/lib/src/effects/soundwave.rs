//! Named sonic-wave recipe: one directional compression ring.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, SoundwavePart};
use crate::layers::soundwave::{self, SOUNDWAVE_DURATION};

/// Authored directional soundwave. Mutating this after spawn does not rebuild
/// the instance. Live Bray rings update [`crate::SoundwaveMaterial`] directly.
#[derive(Component, Clone, Debug)]
pub struct Soundwave {
	pub part: SoundwavePart,
}

impl Soundwave {
	pub fn new(part: SoundwavePart) -> Self {
		Self { part }
	}

	pub fn from_mesh(mesh: Handle<Mesh>) -> Self {
		Self::new(soundwave::compile_mesh(mesh))
	}

	pub fn duration() -> f32 {
		SOUNDWAVE_DURATION
	}
}

impl EffectRecipe for Soundwave {
	fn effect_definition(&self) -> EffectDefinition {
		soundwave::definition(self.part.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::names::SOUNDWAVE;

	#[test]
	fn recipe_keeps_the_canonical_name() {
		let recipe = Soundwave::from_mesh(Handle::default());
		assert_eq!(recipe.effect_definition().name, SOUNDWAVE);
	}
}
