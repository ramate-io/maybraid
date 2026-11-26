//! Named dust composite: one packaged flipbook layer.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, FlipbookPart};
use crate::layers::dust::DustLayer;

/// Authored dust recipe. Mutating this after spawn does not rebuild the instance.
#[derive(Component, Clone, Debug)]
pub struct Dust {
	pub layer: DustLayer,
}

impl Dust {
	pub const ROLLING_WAVE: &'static str = "vfx/dust/rolling_wave.flipbook.json";

	pub fn new(layer: DustLayer) -> Self {
		Self { layer }
	}

	pub fn rolling_wave() -> Self {
		Self::new(DustLayer::new(FlipbookPart::new(Self::ROLLING_WAVE)))
	}
}

impl Default for Dust {
	fn default() -> Self {
		Self::rolling_wave()
	}
}

impl EffectRecipe for Dust {
	fn effect_definition(&self) -> EffectDefinition {
		self.layer.effect_definition()
	}
}
