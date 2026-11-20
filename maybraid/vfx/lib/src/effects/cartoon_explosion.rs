//! Explicit authored Cartoon smoke explosion recipe.
use crate::composition::{CartoonExplosionPart, EffectDefinition, EffectRecipe};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CartoonExplosion {
	pub part: CartoonExplosionPart,
}
impl CartoonExplosion {
	pub fn load(s: &AssetServer) -> Self {
		Self { part: CartoonExplosionPart::load(s) }
	}
}
impl EffectRecipe for CartoonExplosion {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			"cartoon-explosion",
			crate::layers::cartoon_explosion::CartoonExplosionLayer { part: self.part.clone() }
				.layers(),
		)
	}
}
