//! Reusable distance-kick-up recipe; producers supply motion through `herd::Inputs`.
use crate::{
	composition::{CartoonLandingPart, EffectDefinition, EffectRecipe, HerdPart},
	layers::herd::HerdLayer,
	parts::herd::Settings,
};
use bevy::prelude::*;
#[derive(Clone, Debug, Component)]
pub struct HerdDust {
	pub layer: HerdLayer,
}
impl HerdDust {
	pub fn load(server: &AssetServer, settings: Settings) -> Self {
		Self {
			layer: HerdLayer {
				part: HerdPart {
					meshes: CartoonLandingPart::load(server).meshes,
					assets: server.load("vfx/dust/combined.combined.json"),
					settings,
				},
			},
		}
	}
}
impl EffectRecipe for HerdDust {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new("herd-dust", [self.layer.layer()])
	}
}
