//! Glowing wisps converge before a combat body appears.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, MeshPart, ParticlePart};
use crate::layers::spirit_spawn;

#[derive(Component, Clone, Debug)]
pub struct SpiritSpawn {
	pub core: MeshPart,
	pub wisps: ParticlePart,
}

impl SpiritSpawn {
	pub fn new(core: MeshPart, wisps: ParticlePart) -> Self {
		Self { core, wisps }
	}
}

impl EffectRecipe for SpiritSpawn {
	fn effect_definition(&self) -> EffectDefinition {
		spirit_spawn::definition(self.core.clone(), self.wisps.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::ParticleShade;
	use crate::names::SPIRIT_SPAWN;

	#[test]
	fn recipe_names_spawn_composite() {
		let core = MeshPart {
			name: SPIRIT_SPAWN.into(),
			mesh: Handle::default(),
			kind: crate::composition::LobeKind::Flash,
			lobes: Vec::new(),
			duration: spirit_spawn::SPIRIT_SPAWN_MESH_FADE,
		};
		let wisps = ParticlePart {
			name: SPIRIT_SPAWN.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: spirit_spawn::SPIRIT_SPAWN_LIFE_MAX,
			shade: ParticleShade::Spirit,
		};
		let recipe = SpiritSpawn::new(core, wisps);
		assert_eq!(recipe.effect_definition().name, SPIRIT_SPAWN);
		assert_eq!(recipe.effect_definition().layers.len(), 3);
	}
}
