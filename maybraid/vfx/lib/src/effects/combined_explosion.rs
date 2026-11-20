//! Explicit authored Combined smoke explosion recipe.
use crate::composition::{CombinedExplosionPart, EffectDefinition, EffectRecipe};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CombinedExplosion {
	pub part: CombinedExplosionPart,
}
impl CombinedExplosion {
	pub fn load(s: &AssetServer) -> Self {
		Self { part: CombinedExplosionPart::load(s) }
	}
}
impl EffectRecipe for CombinedExplosion {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			"combined-explosion",
			crate::layers::combined_explosion::CombinedExplosionLayer { part: self.part.clone() }
				.layers(),
		)
	}
}
