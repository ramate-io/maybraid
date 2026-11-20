//! Independent timed authored Cartoon layer.
use crate::composition::{CartoonExplosionPart, EffectLayer, EffectPart};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CartoonExplosionLayer {
	pub part: CartoonExplosionPart,
}
impl CartoonExplosionLayer {
	pub fn layers(&self) -> [EffectLayer; 1] {
		[EffectLayer {
			part: EffectPart::CartoonExplosion(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}]
	}
}
