//! Corpse dissolution into an outward energy burst.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, MeshPart, ParticlePart};
use crate::layers::spirit;

#[derive(Component, Clone, Debug)]
pub struct SpiritDissolve {
	pub shell: MeshPart,
	pub streaks: ParticlePart,
	pub motes: ParticlePart,
	pub embers: ParticlePart,
}

impl SpiritDissolve {
	pub fn new(
		shell: MeshPart,
		streaks: ParticlePart,
		motes: ParticlePart,
		embers: ParticlePart,
	) -> Self {
		Self { shell, streaks, motes, embers }
	}
}

impl EffectRecipe for SpiritDissolve {
	fn effect_definition(&self) -> EffectDefinition {
		spirit::definition(
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
	use crate::names::SPIRIT_DISSOLVE;

	#[test]
	fn recipe_matches_standalone_layer() {
		let mesh = MeshPart {
			name: SPIRIT_DISSOLVE.into(),
			mesh: Handle::default(),
			kind: crate::composition::LobeKind::Flash,
			lobes: Vec::new(),
			duration: 1.0,
		};
		let particle = ParticlePart {
			name: SPIRIT_DISSOLVE.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: 1.0,
			shade: crate::composition::ParticleShade::Spirit,
		};
		let recipe = SpiritDissolve::new(mesh.clone(), particle.clone(), particle.clone(), particle);
		assert_eq!(recipe.effect_definition().name, SPIRIT_DISSOLVE);
	}
}
