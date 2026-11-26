//! Generic parts, layers, and composites. Preset names live in [`crate::library`].

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::flipbook_ref::FlipbookRef;

/// Which explosion colors a particle layer samples from the instance palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleShade {
	Fire,
	Smoke,
	Spark,
}

/// Shared Hanabi effect handle and configurable spawn defaults.
#[derive(Clone, Debug)]
pub struct ParticlePart {
	pub name: String,
	pub effect: Handle<EffectAsset>,
	pub images: Vec<Handle<Image>>,
	pub count: f32,
	pub capacity: u32,
	pub max_lifetime: f32,
	pub shade: ParticleShade,
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

impl LobeSpec {
	pub fn new(offset: Vec3, scale: Vec3) -> Self {
		Self { offset, scale, euler: Vec3::ZERO, expand: 0.7, rise: 0.0, roll: 0.0, duration: 0.5 }
	}

	pub fn with_euler(mut self, euler: Vec3) -> Self {
		self.euler = euler;
		self
	}

	pub fn with_expand(mut self, expand: f32) -> Self {
		self.expand = expand;
		self
	}

	pub fn with_rise(mut self, rise: f32) -> Self {
		self.rise = rise;
		self
	}

	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	pub fn with_duration(mut self, duration: f32) -> Self {
		self.duration = duration;
		self
	}
}

/// Shared mesh plus a lobe arrangement. Runtime animation stays in [`crate::parts::lobe`].
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

/// Driven or recipe-spawned directional ring. Runtime pose lives in [`crate::parts::soundwave`].
#[derive(Clone, Debug)]
pub struct SoundwavePart {
	pub name: String,
	pub mesh: Handle<Mesh>,
	pub duration: f32,
}

/// Authored flipbook identity. The runtime supplies its mesh, handle, and manifest timing.
#[derive(Clone, Debug)]
pub struct FlipbookPart {
	pub flipbook: FlipbookRef,
}

impl FlipbookPart {
	pub fn new(flipbook: impl Into<FlipbookRef>) -> Self {
		Self { flipbook: flipbook.into() }
	}
}

/// Concrete particle, light, mesh, or packaged flipbook variant owned by a layer.
#[derive(Clone, Debug)]
pub enum EffectPart {
	Flipbook(FlipbookPart),
	Particle(ParticlePart),
	Light(LightPulse),
	Mesh(MeshPart),
	Soundwave(SoundwavePart),
}

/// One timed piece of a composite: part, delay, local pose, scale.
#[derive(Clone, Debug)]
pub struct EffectLayer {
	/// Optional author-facing label for diagnostics and the preview inspector.
	pub name: Option<String>,
	pub part: EffectPart,
	pub delay: f32,
	pub transform: Transform,
	pub scale: f32,
}

impl EffectLayer {
	/// A packaged flipbook needs only its asset-relative manifest path.
	pub fn flipbook_path(path: impl Into<FlipbookRef>) -> Self {
		Self::flipbook(FlipbookPart::new(path))
	}

	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	/// Replace translation while preserving rotation and axis scale.
	pub fn with_translation(mut self, translation: Vec3) -> Self {
		self.transform.translation = translation;
		self
	}

	/// Replace rotation while preserving translation and axis scale.
	pub fn with_rotation(mut self, rotation: Quat) -> Self {
		self.transform.rotation = rotation;
		self
	}

	pub fn into_group(self) -> EffectLayerGroup {
		EffectLayerGroup::new([self])
	}

	pub fn label(&self) -> &str {
		self.name.as_deref().unwrap_or_else(|| match &self.part {
			EffectPart::Flipbook(part) => &part.flipbook.path,
			EffectPart::Particle(part) => &part.name,
			EffectPart::Light(_) => "light",
			EffectPart::Mesh(part) => &part.name,
			EffectPart::Soundwave(part) => &part.name,
		})
	}

	pub fn flipbook(part: FlipbookPart) -> Self {
		Self {
			name: None,
			part: EffectPart::Flipbook(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn particle(part: ParticlePart) -> Self {
		Self {
			name: None,
			part: EffectPart::Particle(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn light(pulse: LightPulse) -> Self {
		Self {
			name: None,
			part: EffectPart::Light(pulse),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn mesh(part: MeshPart) -> Self {
		Self {
			name: None,
			part: EffectPart::Mesh(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn soundwave(part: SoundwavePart) -> Self {
		Self {
			name: None,
			part: EffectPart::Soundwave(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

	pub fn with_delay(mut self, delay: f32) -> Self {
		self.delay = delay;
		self
	}

	/// Uniform size multiplier, multiplied by `transform.scale`; does not move the layer.
	pub fn with_scale(mut self, scale: f32) -> Self {
		self.scale = scale;
		self
	}

	pub fn with_transform(mut self, transform: Transform) -> Self {
		self.transform = transform;
		self
	}

	pub fn duration(&self) -> f32 {
		self.delay + self.part_lifetime()
	}

	pub fn part_lifetime(&self) -> f32 {
		match &self.part {
			// Unknown until resolved; runtime layer life gates completion.
			EffectPart::Flipbook(_) => 0.0,
			EffectPart::Particle(part) => part.max_lifetime,
			EffectPart::Light(pulse) => pulse.fade,
			EffectPart::Mesh(part) => part.duration,
			EffectPart::Soundwave(part) => part.duration,
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
	pub fn from_groups(
		name: impl Into<String>,
		groups: impl IntoIterator<Item = EffectLayerGroup>,
	) -> Self {
		Self::new(name, groups.into_iter().flatten())
	}

	pub fn new(name: impl Into<String>, layers: impl IntoIterator<Item = EffectLayer>) -> Self {
		Self { name: name.into(), layers: layers.into_iter().collect() }
	}

	/// Planned duration; flipbook lifetimes are filled from manifests at runtime.
	pub fn duration(&self) -> f32 {
		self.layers.iter().map(EffectLayer::duration).fold(0.0, f32::max)
	}
}

/// Authored configuration that lowers into a flattened [`EffectDefinition`].
pub trait EffectRecipe {
	fn effect_definition(&self) -> EffectDefinition;
}

/// Reusable authored layers, flattened before spawning. Operations compose in call order.
/// Uniform group scaling also scales offsets. Restricting it to a scalar avoids shear
/// when composing rotated layers with nonuniform local scale.
#[derive(Clone, Debug, Default)]
pub struct EffectLayerGroup {
	layers: Vec<EffectLayer>,
}

impl EffectLayerGroup {
	pub fn new(layers: impl IntoIterator<Item = EffectLayer>) -> Self {
		Self { layers: layers.into_iter().collect() }
	}

	pub fn from_groups(groups: impl IntoIterator<Item = Self>) -> Self {
		Self::new(groups.into_iter().flatten())
	}

	/// Add a delay to every layer, preserving relative timing.
	pub fn delayed(mut self, seconds: f32) -> Self {
		for layer in &mut self.layers {
			layer.delay += seconds;
		}
		self
	}

	pub fn translated(mut self, offset: Vec3) -> Self {
		for layer in &mut self.layers {
			layer.transform.translation += offset;
		}
		self
	}

	pub fn rotated(mut self, rotation: Quat) -> Self {
		for layer in &mut self.layers {
			layer.transform.translation = rotation * layer.transform.translation;
			layer.transform.rotation = rotation * layer.transform.rotation;
		}
		self
	}

	pub fn scaled(mut self, scale: f32) -> Self {
		for layer in &mut self.layers {
			layer.transform.translation *= scale;
			layer.scale *= scale;
		}
		self
	}
}

impl IntoIterator for EffectLayerGroup {
	type Item = EffectLayer;
	type IntoIter = std::vec::IntoIter<EffectLayer>;
	fn into_iter(self) -> Self::IntoIter {
		self.layers.into_iter()
	}
}

#[cfg(test)]
mod tests;
