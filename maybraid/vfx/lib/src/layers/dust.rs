//! Ground-aligned packaged flipbook layer. Standalone or composed into [`crate::effects::dust::Dust`].

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectLayer, EffectRecipe, FlipbookPart};
use crate::names::DUST;

/// Reusable dust layer: one capture-plane flipbook part plus timing/pose.
#[derive(Component, Clone, Debug, Default)]
pub struct DustLayer {
	pub part: FlipbookPart,
	pub delay: f32,
	pub transform: Transform,
	pub scale: f32,
}

impl DustLayer {
	pub fn new(part: FlipbookPart) -> Self {
		Self { part, delay: 0.0, transform: Transform::IDENTITY, scale: 1.0 }
	}

	pub fn effect_layers(&self) -> [EffectLayer; 1] {
		[EffectLayer::flipbook(self.part.clone())
			.with_delay(self.delay)
			.with_transform(self.transform)
			.with_scale(self.scale)]
	}
}

impl EffectRecipe for DustLayer {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(DUST, self.effect_layers())
	}
}
