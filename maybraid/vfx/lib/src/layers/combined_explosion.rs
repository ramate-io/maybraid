//! Independent timed authored Combined layer.
use crate::composition::{CombinedExplosionPart, EffectLayer, EffectPart};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct CombinedExplosionLayer {
	pub part: CombinedExplosionPart,
}
impl CombinedExplosionLayer {
	pub fn layers(&self) -> [EffectLayer; 1] {
		[EffectLayer {
			part: EffectPart::CombinedExplosion(self.part.clone()),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}]
	}
}
