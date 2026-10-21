//! Generic parts, layers, and composites. Preset names live in [`crate::library`].

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

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

/// Shared mesh plus a lobe arrangement. Runtime animation stays in [`crate::lobes`].
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

/// Default per-instance scale clamp for standalone layers and composites.
pub const DEFAULT_SCALE_MIN: f32 = 0.25;
pub const DEFAULT_SCALE_MAX: f32 = 4.0;

/// Per-definition scale clamp applied at spawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleBounds {
	pub min: f32,
	pub max: f32,
}

/// Tweakable look knobs for [`EffectKind::SonicWave`]. Gameplay pose still comes from
/// [`crate::drive::SonicWaveState`]; these only affect mesh materials and trail ribbons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SonicWaveVisual {
	pub ring_gain: f32,
	pub ring_softness: f32,
	pub trail_ribbons_per_ring: u32,
	pub trail_history_samples: u32,
	pub trail_max_length: f32,
}

impl Default for SonicWaveVisual {
	fn default() -> Self {
		Self {
			ring_gain: 1.0,
			ring_softness: 0.22,
			trail_ribbons_per_ring: 4,
			trail_history_samples: 14,
			trail_max_length: 2.8,
		}
	}
}

/// Tweakable look knobs for [`EffectKind::GodBeam`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GodBeamVisual {
	pub core_gain: f32,
	pub sheath_gain: f32,
	pub contact_gain: f32,
}

impl Default for GodBeamVisual {
	fn default() -> Self {
		Self { core_gain: 1.0, sheath_gain: 1.0, contact_gain: 1.0 }
	}
}

/// Driven-effect presentation overrides stored on [`EffectDefinition`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct DrivenFieldVisual {
	pub sonic: SonicWaveVisual,
	pub god_beam: GodBeamVisual,
}

impl Default for ScaleBounds {
	fn default() -> Self {
		Self { min: DEFAULT_SCALE_MIN, max: DEFAULT_SCALE_MAX }
	}
}

/// How [`crate::spawn_vfx`] realizes the definition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectKind {
	#[default]
	Burst,
	GodBeam,
	SonicWave,
}

/// Ordered collection of layers forming a named composite.
#[derive(Clone, Debug)]
pub struct EffectDefinition {
	pub name: String,
	pub layers: Vec<EffectLayer>,
	pub scale_bounds: ScaleBounds,
	pub kind: EffectKind,
	pub authored_duration: Option<f32>,
	/// Shared procedural meshes for driven effects. Burst definitions leave this empty.
	pub meshes: Vec<Handle<Mesh>>,
	pub field_visual: DrivenFieldVisual,
}

impl EffectDefinition {
	pub fn new(name: impl Into<String>, layers: impl IntoIterator<Item = EffectLayer>) -> Self {
		Self {
			name: name.into(),
			layers: layers.into_iter().collect(),
			scale_bounds: ScaleBounds::default(),
			kind: EffectKind::Burst,
			authored_duration: None,
			meshes: Vec::new(),
			field_visual: DrivenFieldVisual::default(),
		}
	}

	pub fn with_scale_bounds(mut self, bounds: ScaleBounds) -> Self {
		self.scale_bounds = bounds;
		self
	}

	pub fn with_kind(mut self, kind: EffectKind) -> Self {
		self.kind = kind;
		self
	}

	pub fn with_duration(mut self, duration: f32) -> Self {
		self.authored_duration = Some(duration);
		self
	}

	pub fn with_meshes(mut self, meshes: impl IntoIterator<Item = Handle<Mesh>>) -> Self {
		self.meshes = meshes.into_iter().collect();
		self
	}

	pub fn with_field_visual(mut self, field_visual: DrivenFieldVisual) -> Self {
		self.field_visual = field_visual;
		self
	}

	pub fn duration(&self) -> f32 {
		self.authored_duration
			.unwrap_or_else(|| self.layers.iter().map(EffectLayer::duration).fold(0.0, f32::max))
	}
}
