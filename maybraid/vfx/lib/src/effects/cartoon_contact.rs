//! Typed access to the existing approved contact configuration, without changing it.
use crate::{
	composition::{CartoonFlowPart, EffectDefinition, EffectRecipe},
	layers::cartoon_contact::CartoonContactLayer,
};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CartoonContact {
	pub layer: CartoonContactLayer,
}
impl CartoonContact {
	pub fn landing(server: &AssetServer) -> Self {
		Self::load(server, true)
	}
	pub fn wave(server: &AssetServer) -> Self {
		Self::load(server, false)
	}
	fn load(server: &AssetServer, landing: bool) -> Self {
		Self {
			layer: CartoonContactLayer {
				part: CartoonFlowPart {
					meshes: crate::composition::CartoonLandingPart::load(server).meshes,
					landing,
					refined: true,
				},
			},
		}
	}
}
impl EffectRecipe for CartoonContact {
	fn effect_definition(&self) -> EffectDefinition {
		self.layer.effect_definition()
	}
}
