//! Converging energy shell and streaks before a body materializes.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, MeshPart, ParticlePart};
use crate::layers::spirit_spawn;

#[derive(Component, Clone, Debug)]
pub struct SpiritSpawn {
	pub shell: MeshPart,
	pub streaks: ParticlePart,
	pub motes: ParticlePart,
	pub embers: ParticlePart,
}

impl SpiritSpawn {
	pub fn new(
		shell: MeshPart,
		streaks: ParticlePart,
		motes: ParticlePart,
		embers: ParticlePart,
	) -> Self {
		Self { shell, streaks, motes, embers }
	}
}

impl EffectRecipe for SpiritSpawn {
	fn effect_definition(&self) -> EffectDefinition {
		spirit_spawn::definition(
			self.shell.clone(),
			self.streaks.clone(),
			self.motes.clone(),
			self.embers.clone(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::names::SPIRIT_SPAWN;

	#[test]
	fn recipe_names_spawn_composite() {
		let mesh = MeshPart {
			name: SPIRIT_SPAWN.into(),
			mesh: Handle::default(),
			kind: crate::composition::LobeKind::Flash,
			lobes: Vec::new(),
			duration: spirit_spawn::SPIRIT_SPAWN_SECS,
		};
		let particle = ParticlePart {
			name: SPIRIT_SPAWN.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: spirit_spawn::SPIRIT_SPAWN_SECS,
			shade: crate::composition::ParticleShade::Spirit,
		};
		let recipe = SpiritSpawn::new(mesh, particle.clone(), particle.clone(), particle);
		assert_eq!(recipe.effect_definition().name, SPIRIT_SPAWN);
		assert!(recipe.effect_definition().layers.len() >= 5);
	}
}
