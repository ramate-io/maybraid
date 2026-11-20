//! Ground-pivot Cartoon wave recipe. The packaged flipbook Dust is unchanged.
use crate::{
	composition::{CartoonWavePart, EffectDefinition, EffectRecipe},
	layers::cartoon_wave::CartoonWaveLayer,
};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CartoonWave {
	pub layer: CartoonWaveLayer,
}
impl CartoonWave {
	pub fn load(server: &AssetServer) -> Self {
		Self { layer: CartoonWaveLayer { part: CartoonWavePart::load(server) } }
	}
}
impl EffectRecipe for CartoonWave {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new("cartoon-wave", self.layer.layers())
	}
}
