//! A visual responsibility within the shared Combined instance simulation.
use crate::composition::{CombinedPart, EffectDefinition, EffectLayer, EffectPart, EffectRecipe};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CombinedLayer {
	pub part: CombinedPart,
}
impl CombinedLayer {
	pub fn layer(&self) -> EffectLayer {
		EffectLayer {
			part: EffectPart::Combined(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}
	}
}
impl EffectRecipe for CombinedLayer {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new("combined-dust", [self.layer()])
	}
}
