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

/// Near-core mesh material kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobeKind {
	Fire,
	Smoke,
	Flash,
}

impl LobeKind {
	pub fn as_f32(self) -> f32 {
		match self {
			Self::Fire => 0.0,
			Self::Smoke => 1.0,
			Self::Flash => 2.0,
		}
	}
}

/// One overlapping irregular volume in a mesh cluster.
#[derive(Clone, Copy, Debug)]
pub struct LobeSpec {
	pub offset: Vec3,
	pub scale: Vec3,
	pub euler: Vec3,
	pub expand: f32,
	pub rise: f32,
	pub roll: f32,
	pub duration: f32,
}

/// Shared mesh plus a lobe arrangement. Runtime animation stays in spawn.
#[derive(Clone, Debug)]
pub struct MeshPart {
	pub name: String,
	pub mesh: Handle<Mesh>,
	pub kind: LobeKind,
	pub lobes: Vec<LobeSpec>,
	pub duration: f32,
}

impl MeshPart {
	pub fn duration_from_lobes(
		name: impl Into<String>,
		mesh: Handle<Mesh>,
		kind: LobeKind,
		lobes: Vec<LobeSpec>,
	) -> Self {
		let duration = lobes.iter().map(|lobe| lobe.duration).fold(0.0, f32::max);
		Self { name: name.into(), mesh, kind, lobes, duration }
	}
}

/// Concrete particle, light, or mesh variant owned by a layer.
#[derive(Clone, Debug)]
pub enum EffectPart {
	Particle(ParticlePart),
	Light(LightPulse),
	Mesh(MeshPart),
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

	pub fn mesh(part: MeshPart) -> Self {
		Self {
			part: EffectPart::Mesh(part),
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
		self.delay + self.part_lifetime()
	}

	pub fn part_lifetime(&self) -> f32 {
		match &self.part {
			EffectPart::Particle(part) => part.max_lifetime,
			EffectPart::Light(pulse) => pulse.fade,
			EffectPart::Mesh(part) => part.duration,
		}
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
	pub spark: Handle<Image>,
}
