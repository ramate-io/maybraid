//! The dynamic Combined recipe is configuration; state lives on its VFX instance.
use crate::{
	composition::{CombinedOptions, CombinedPart, CombinedRole, EffectDefinition, EffectRecipe},
	layers::combined::CombinedLayer,
	parts::combined::assets::CombinedAsset,
};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CombinedDust {
	pub assets: Handle<CombinedAsset>,
	pub options: CombinedOptions,
}
impl CombinedDust {
	pub fn load(server: &AssetServer) -> Self {
		Self { assets: server.load("vfx/dust/combined.combined.json"), options: default() }
	}
}
impl EffectRecipe for CombinedDust {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			if self.options.motion == crate::composition::CombinedMotion::FlowGuided {
				"flow-guided-dust"
			} else if self.options.motion.interactive() {
				"interactive-dust"
			} else {
				"combined-dust"
			},
			[CombinedRole::Body, CombinedRole::Clouds, CombinedRole::Grains].map(|role| {
				CombinedLayer {
					part: CombinedPart { assets: self.assets.clone(), role, options: self.options },
				}
				.layer()
			}),
		)
	}
}
