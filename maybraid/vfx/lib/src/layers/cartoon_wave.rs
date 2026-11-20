//! One brief directional wave layer, configured independently of runtime state.
use crate::composition::{CartoonWavePart, EffectLayer, EffectPart};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CartoonWaveLayer {
	pub part: CartoonWavePart,
}
impl CartoonWaveLayer {
	pub fn layers(&self) -> [EffectLayer; 1] {
		[EffectLayer {
			part: EffectPart::CartoonWave(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}]
	}
}

impl crate::composition::EffectRecipe for CartoonWaveLayer {
	fn effect_definition(&self) -> crate::composition::EffectDefinition {
		crate::composition::EffectDefinition::new("cartoon-wave", self.layers())
	}
}
