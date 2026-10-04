//! Shared definitions: parts, layers, and named composites.

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::assets::FlipbookAsset;

pub const FIREY_EXPLOSION: &str = "firey_explosion";
pub const FLASH: &str = "flash";
pub const FIREBALL: &str = "fireball";
pub const SMOKE: &str = "smoke";
pub const SPARKS: &str = "sparks";

/// Shared Hanabi effect handle and configurable spawn defaults.
#[derive(Clone, Debug)]
pub struct ParticlePart {
	pub name: String,
	pub effect: Handle<EffectAsset>,
	pub images: Vec<Handle<Image>>,
	pub count: f32,
	pub capacity: u32,
	pub max_lifetime: f32,
}

/// Light color, peak intensity, range, and fade duration.
#[derive(Clone, Copy, Debug)]
pub struct LightPulse {
	pub color: Color,
	pub peak_intensity: f32,
	pub range: f32,
	pub fade: f32,
}

/// Concrete particle or light variant owned by a layer.
#[derive(Clone, Debug)]
pub enum EffectPart {
	Particle(ParticlePart),
	Light(LightPulse),
}

/// One timed piece of a composite: part, delay, local pose, scale.
#[derive(Clone, Debug)]
pub struct EffectLayer {
	pub part: EffectPart,
	pub delay: f32,
	pub transform: Transform,
	pub scale: f32,
}

impl EffectLayer {
	pub fn particle(part: ParticlePart) -> Self {
		Self {
			part: EffectPart::Particle(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn light(pulse: LightPulse) -> Self {
		Self {
			part: EffectPart::Light(pulse),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn with_delay(mut self, delay: f32) -> Self {
		self.delay = delay;
		self
	}

	pub fn with_scale(mut self, scale: f32) -> Self {
		self.scale = scale;
		self
	}

	pub fn duration(&self) -> f32 {
		let life = match &self.part {
			EffectPart::Particle(part) => part.max_lifetime,
			EffectPart::Light(pulse) => pulse.fade,
		};
		self.delay + life
	}
}

/// Ordered collection of layers forming a named composite.
#[derive(Clone, Debug)]
pub struct EffectDefinition {
	pub name: String,
	pub layers: Vec<EffectLayer>,
}

impl EffectDefinition {
	pub fn new(name: impl Into<String>, layers: impl IntoIterator<Item = EffectLayer>) -> Self {
		Self { name: name.into(), layers: layers.into_iter().collect() }
	}

	pub fn duration(&self) -> f32 {
		self.layers.iter().map(EffectLayer::duration).fold(0.0, f32::max)
	}
}

/// Flipbooks compiled for the first explosion set.
#[derive(Clone, Debug)]
pub struct VfxFlipbooks {
	pub fire: FlipbookAsset,
	pub smoke: FlipbookAsset,
}
