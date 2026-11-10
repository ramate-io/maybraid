//! Corpse dissolution into celestial motes.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, MeshPart, ParticlePart};
use crate::layers::spirit;
#[derive(Component, Clone, Debug)]
pub struct SpiritDissolve {
	pub core: MeshPart,
	pub wisps: ParticlePart,
}

impl SpiritDissolve {
	pub fn new(core: MeshPart, wisps: ParticlePart) -> Self {
		Self { core, wisps }
	}
}

impl EffectRecipe for SpiritDissolve {
	fn effect_definition(&self) -> EffectDefinition {
		spirit::definition(self.core.clone(), self.wisps.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::ParticleShade;
	use crate::names::SPIRIT_DISSOLVE;

	#[test]
	fn recipe_matches_standalone_layer() {
		let wisps = ParticlePart {
			name: SPIRIT_DISSOLVE.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: spirit::SPIRIT_LIFE_MAX,
			shade: ParticleShade::Spirit,
		};
		let core = MeshPart {
			name: SPIRIT_DISSOLVE.into(),
			mesh: Handle::default(),
			kind: crate::composition::LobeKind::Flash,
			lobes: Vec::new(),
			duration: spirit::SPIRIT_MESH_FADE,
		};
		let recipe = SpiritDissolve::new(core, wisps);
		assert_eq!(recipe.effect_definition().name, SPIRIT_DISSOLVE);
		assert!(
			(recipe.effect_definition().duration() - spirit::SPIRIT_LIFE_MAX).abs() < 1e-4,
			"duration {}",
			recipe.effect_definition().duration()
		);
	}
}
