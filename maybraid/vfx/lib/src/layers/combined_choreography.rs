use crate::composition::{
	CombinedChoreographyPart, EffectDefinition, EffectLayer, EffectPart, EffectRecipe,
};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CombinedChoreographyLayer {
	pub part: CombinedChoreographyPart,
}
impl EffectRecipe for CombinedChoreographyLayer {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			match (self.part.landing, self.part.contact) {
				(true, false) => "combined-landing",
				(true, true) => "combined-landing-contact",
				_ => "combined-wave-contact",
			},
			[EffectLayer {
				part: EffectPart::CombinedChoreography(self.part.clone()),
				delay: 0.,
				transform: Transform::IDENTITY,
				scale: 1.,
			}],
		)
	}
}
