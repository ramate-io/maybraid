//! Named dust composite: one packaged flipbook layer.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectRecipe, FlipbookPart};
use crate::layers::dust::DustLayer;

/// Authored dust recipe. Mutating this after spawn does not rebuild the instance.
#[derive(Component, Clone, Debug, Default)]
pub struct Dust {
	pub layer: DustLayer,
}

impl Dust {
	pub const ROLLING_WAVE: &'static str = "vfx/dust/rolling_wave.flipbook.json";
	pub const ROLLING_WAVE_DURATION: f32 = 48.0 / 15.0;

	pub fn new(layer: DustLayer) -> Self {
		Self { layer }
	}

	pub fn rolling_wave(server: &AssetServer, meshes: &mut Assets<Mesh>) -> Self {
		Self::new(DustLayer::new(FlipbookPart::load(
			server,
			Self::ROLLING_WAVE,
			meshes.add(Rectangle::default()),
			Self::ROLLING_WAVE_DURATION,
		)))
	}
}

impl EffectRecipe for Dust {
	fn effect_definition(&self) -> EffectDefinition {
		self.layer.effect_definition()
	}
}
