use crate::composition::{
	CartoonFlowPart, EffectDefinition, EffectLayer, EffectPart, EffectRecipe,
};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CartoonContactLayer {
	pub part: CartoonFlowPart,
}
impl EffectRecipe for CartoonContactLayer {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			if self.part.landing { "cartoon-landing-contact" } else { "cartoon-wave-contact" },
			[EffectLayer {
				part: EffectPart::CartoonFlow(self.part.clone()),
				delay: 0.,
				transform: Transform::IDENTITY,
				scale: 1.,
			}],
		)
	}
}
