//! Generic parts, layers, and composites. Preset names live in [`crate::library`].

use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::parts::packaged::PackagedFlipbook;

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

/// Shared packaged flipbook handle and proxy mesh. Duration is a fallback until the manifest loads.
#[derive(Clone, Debug, Default)]
pub struct FlipbookPart {
	pub flipbook: Handle<PackagedFlipbook>,
	pub mesh: Handle<Mesh>,
	pub duration: f32,
}

impl FlipbookPart {
	pub fn load(
		server: &AssetServer,
		path: impl Into<String>,
		mesh: Handle<Mesh>,
		duration: f32,
	) -> Self {
		Self { flipbook: server.load(path.into()), mesh, duration }
	}
}

/// Shared authored body meshes for one portable landing emitter.
#[derive(Clone, Debug)]
pub struct CartoonLandingPart {
	pub meshes: [Handle<Mesh>; 3],
}

/// Shared reusable meshes for the finite four-second directional wave.
#[derive(Clone, Debug)]
pub struct CartoonWavePart {
	pub meshes: [Handle<Mesh>; 3],
}

/// Combined roles share one simulation on the owning instance and one optical composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombinedRole {
	Body,
	Clouds,
	Grains,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombinedScenario {
	#[default]
	Normal,
	WindTurn,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombinedLayers {
	#[default]
	All,
	Body,
	Clouds,
	Grains,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombinedQuality {
	#[default]
	Default,
	Low,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombinedMotion {
	#[default]
	Reference,
	InteractiveBrief,
	InteractiveContinuous,
	FlowGuided,
}
impl CombinedMotion {
	pub fn duration(self) -> f32 {
		match self {
			Self::Reference => 4.7,
			Self::InteractiveBrief | Self::FlowGuided => 14.,
			Self::InteractiveContinuous => 56.,
		}
	}
	pub fn interactive(self) -> bool {
		self != Self::Reference
	}
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CombinedOptions {
	pub scenario: CombinedScenario,
	pub motion: CombinedMotion,
	pub layers: CombinedLayers,
	pub quality: CombinedQuality,
}
#[derive(Clone, Debug)]
pub struct CombinedPart {
	pub assets: Handle<crate::parts::combined::assets::CombinedAsset>,
	pub role: CombinedRole,
	pub options: CombinedOptions,
}

#[derive(Clone, Debug)]
pub enum EffectPart {
	CombinedChoreography(CombinedChoreographyPart),
	CartoonExplosion(CartoonExplosionPart),
	CombinedExplosion(CombinedExplosionPart),
	Herd(HerdPart),
	CartoonFlow(CartoonFlowPart),
	CartoonLanding(CartoonLandingPart),
	CartoonWave(CartoonWavePart),
	Combined(CombinedPart),
	Flipbook(FlipbookPart),
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
	pub fn flipbook(part: FlipbookPart) -> Self {
		Self {
			part: EffectPart::Flipbook(part),
			delay: 0.0,
			transform: Transform::IDENTITY,
			scale: 1.0,
		}
	}

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

	pub fn with_transform(mut self, transform: Transform) -> Self {
		self.transform = transform;
		self
	}

	pub fn duration(&self) -> f32 {
		self.delay + self.part_lifetime()
	}

	pub fn part_lifetime(&self) -> f32 {
		match &self.part {
			EffectPart::CartoonLanding(_) => 1.2,
			EffectPart::Herd(_) => f32::MAX,
			EffectPart::CombinedChoreography(p) => p.duration(),
			EffectPart::CartoonExplosion(_) | EffectPart::CombinedExplosion(_) => 14.,
			EffectPart::CartoonWave(_) => 4.0,
			EffectPart::CartoonFlow(p) => {
				if p.landing {
					1.2
				} else {
					4.
				}
			}
			EffectPart::Combined(part) => part.options.motion.duration(),
			EffectPart::Flipbook(part) => part.duration,
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

impl Default for ScaleBounds {
	fn default() -> Self {
		Self { min: DEFAULT_SCALE_MIN, max: DEFAULT_SCALE_MAX }
	}
}

/// Ordered collection of layers forming a named composite.
#[derive(Clone, Debug)]
pub struct EffectDefinition {
	pub name: String,
	pub layers: Vec<EffectLayer>,
	pub scale_bounds: ScaleBounds,
}

impl EffectDefinition {
	pub fn new(name: impl Into<String>, layers: impl IntoIterator<Item = EffectLayer>) -> Self {
		Self {
			name: name.into(),
			layers: layers.into_iter().collect(),
			scale_bounds: ScaleBounds::default(),
		}
	}

	pub fn with_scale_bounds(mut self, bounds: ScaleBounds) -> Self {
		self.scale_bounds = bounds;
		self
	}

	pub fn duration(&self) -> f32 {
		self.layers.iter().map(EffectLayer::duration).fold(0.0, f32::max)
	}
}

/// Authored configuration that lowers into a flattened [`EffectDefinition`].
pub trait EffectRecipe {
	fn effect_definition(&self) -> EffectDefinition;
}

#[derive(Clone, Debug)]
pub struct CartoonFlowPart {
	pub refined: bool,
	pub meshes: [Handle<Mesh>; 3],
	pub landing: bool,
}

/// One shared, bounded herd region. Configuration only; input/state live on the instance.
#[derive(Clone, Debug)]
pub struct HerdPart {
	pub meshes: [Handle<Mesh>; 3],
	pub assets: Handle<crate::parts::combined::assets::CombinedAsset>,
	pub settings: crate::parts::herd::Settings,
}

/// Authored explosion placement and style-local quality; no cross-style fallback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExplosionLod {
	#[default]
	Full,
	Medium,
	Low,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExplosionQuality {
	#[default]
	High,
	Medium,
	Low,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExplosionDiagnostic {
	#[default]
	Composed,
	Main,
	Fine,
	Spikes,
	Section,
}
#[derive(Clone, Debug)]
pub struct CartoonExplosionPart {
	pub track: Handle<crate::parts::smoke_explosion::track::ExplosionTrack>,
	pub heading: Option<Vec3>,
	pub lod: ExplosionLod,
}
#[derive(Clone, Debug)]
pub struct CombinedExplosionPart {
	pub asset: Handle<crate::parts::smoke_explosion::density::CombinedExplosionAsset>,
	pub heading: Option<Vec3>,
	pub quality: ExplosionQuality,
	pub diagnostic: ExplosionDiagnostic,
}

/// Shared Landing/contact motion with independent Combined optical appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChoreographyOptions {
	pub layers: CombinedLayers,
	pub quality: CombinedQuality,
	pub body_concentration: f32,
	pub fine_concentration: f32,
	pub cloud_rate: f64,
	pub grain_rate: f64,
}
impl Default for ChoreographyOptions {
	fn default() -> Self {
		Self {
			layers: CombinedLayers::All,
			quality: CombinedQuality::Default,
			body_concentration: 1.,
			fine_concentration: 1.,
			cloud_rate: 5.,
			grain_rate: 3.,
		}
	}
}
#[derive(Clone, Debug)]
pub struct CombinedChoreographyPart {
	pub assets: Handle<crate::parts::combined::assets::CombinedAsset>,
	pub meshes: [Handle<Mesh>; 3],
	pub landing: bool,
	pub contact: bool,
	pub options: ChoreographyOptions,
}
impl CombinedChoreographyPart {
	pub fn duration(&self) -> f32 {
		if self.landing {
			2.4
		} else {
			5.7
		}
	}
}
