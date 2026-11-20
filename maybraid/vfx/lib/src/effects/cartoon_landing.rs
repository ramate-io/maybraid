//! Ground-pivot Cartoon landing recipe. The packaged flipbook Dust is unchanged.
use crate::{
	composition::{CartoonLandingPart, EffectDefinition, EffectRecipe},
	layers::cartoon_landing::CartoonLandingLayer,
};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CartoonLanding {
	pub layer: CartoonLandingLayer,
}
impl CartoonLanding {
	pub fn load(server: &AssetServer) -> Self {
		Self { layer: CartoonLandingLayer { part: CartoonLandingPart::load(server) } }
	}
}
impl EffectRecipe for CartoonLanding {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new("cartoon-landing", self.layer.layers())
	}
}
