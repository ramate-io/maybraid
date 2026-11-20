//! One brief landing emitter layer, configured independently of runtime state.
use crate::composition::{CartoonLandingPart, EffectLayer, EffectPart};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CartoonLandingLayer {
	pub part: CartoonLandingPart,
}
impl CartoonLandingLayer {
	pub fn layers(&self) -> [EffectLayer; 1] {
		[EffectLayer {
			part: EffectPart::CartoonLanding(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}]
	}
}

impl crate::composition::EffectRecipe for CartoonLandingLayer {
	fn effect_definition(&self) -> crate::composition::EffectDefinition {
		crate::composition::EffectDefinition::new("cartoon-landing", self.layers())
	}
}
