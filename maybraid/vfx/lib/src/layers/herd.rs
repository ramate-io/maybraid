use crate::composition::{EffectLayer, EffectPart, HerdPart};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct HerdLayer {
	pub part: HerdPart,
}
impl HerdLayer {
	pub fn layer(&self) -> EffectLayer {
		EffectLayer {
			part: EffectPart::Herd(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}
	}
}
