//! Presentation cues derived from attack events. Hosts map these to VFX, pad
//! rumble, and animation; they do not apply damage.

use attacks::{
	AttackId, AttackInstance, AttackInstanceId, AttackStage, AttackSystems, PresentationPhase,
	WavefrontPresentation,
};
use bevy::prelude::*;

use crate::{BikanwardaAttackParams, BRAY, GOD_BEAM, PARADE, PARADE_BLAST};

/// Fiery-explosion request. Playground / world maps this onto `maybraid-vfx`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ParadeExplosionCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub position: Vec3,
	pub radius: f32,
}

/// Preset for [#1146](https://github.com/ramate-io/maybraid/pull/1146) `SpatialRumble`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialRumblePreset {
	BrayWave,
	HeavyLanding,
	/// Placeholder-authored radius / intensity / duration.
	Authored,
}

/// Distance-attenuated rumble / shake request. Not a pad pulse by itself.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SpatialRumbleCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub origin: Vec3,
	pub radius: f32,
	pub intensity: f32,
	pub duration: f32,
	pub preset: SpatialRumblePreset,
}

impl SpatialRumbleCue {
	pub fn bray_wave(
		instance: AttackInstanceId,
		owner: Entity,
		origin: Vec3,
		bray: &crate::BrayParams,
	) -> Self {
		Self {
			instance,
			owner,
			origin,
			radius: bray.rumble_radius,
			intensity: bray.rumble_intensity,
			duration: bray.rumble_secs,
			preset: SpatialRumblePreset::BrayWave,
		}
	}

	pub fn heavy_landing(instance: AttackInstanceId, owner: Entity, origin: Vec3) -> Self {
		Self {
			instance,
			owner,
			origin,
			radius: 14.0,
			intensity: 0.9,
			duration: 0.38,
			preset: SpatialRumblePreset::HeavyLanding,
		}
	}
}

/// Separate landing / stomp cue. [#1119](https://github.com/ramate-io/maybraid/issues/1119)
/// asks that any heavy landing stay off the wave rumble channel.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ImpactCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub origin: Vec3,
}

/// Eye-glow for God Beam windup ([#1152](https://github.com/ramate-io/maybraid/pull/1152)).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EyeGlowCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub active: bool,
}

impl EyeGlowCue {
	pub fn god_beam_warning(instance: AttackInstanceId, owner: Entity) -> Self {
		Self { instance, owner, active: true }
	}

	pub fn off(instance: AttackInstanceId, owner: Entity) -> Self {
		Self { instance, owner, active: false }
	}
}

/// Contract mirror for [#1150](https://github.com/ramate-io/maybraid/pull/1150) `god_beam`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct GodBeamVfxCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub phase: PresentationPhase,
	pub hazard_index: usize,
	pub ground: Vec3,
	/// Column **diameter** in meters (`2 *` gameplay cylinder radius).
	pub diameter: f32,
	pub gameplay_height: f32,
	pub active_secs: f32,
}

/// Contract mirror for [#1150](https://github.com/ramate-io/maybraid/pull/1150) `sonic_wave`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SonicWaveVfxCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub phase: PresentationPhase,
	pub hazard_index: usize,
	pub kind: SonicWaveVfxKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SonicWaveVfxKind {
	Spawn {
		origin: Vec3,
		range: f32,
		speed: f32,
		radius: f32,
		thickness: f32,
		wave_count: u8,
		interval: f32,
		pitch: f32,
	},
	Follow(WavefrontPresentation),
	Despawn,
}

/// Contract mirror for [#1141](https://github.com/ramate-io/maybraid/pull/1141) Epiphant Bray clip.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrayAnimationCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub phase: BrayAnimationPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrayAnimationPhase {
	Windup,
	Execute,
	Recover,
	Cancelled,
}

#[derive(Component, Clone, Copy, Debug)]
struct BrayImpactArmed {
	due: f32,
}

pub fn cue_plugin(app: &mut App) {
	app.add_message::<ParadeExplosionCue>()
		.add_message::<SpatialRumbleCue>()
		.add_message::<ImpactCue>()
		.add_message::<EyeGlowCue>()
		.add_message::<GodBeamVfxCue>()
		.add_message::<SonicWaveVfxCue>()
		.add_message::<BrayAnimationCue>()
		.add_systems(
			Update,
			(arm_bray_impact, fire_bray_impact, impact_to_spatial_rumble)
				.chain()
				.after(AttackSystems::Advance),
		);
}

fn arm_bray_impact(
	params: Res<BikanwardaAttackParams>,
	instances: Query<(Entity, &AttackInstance), Without<BrayImpactArmed>>,
	mut commands: Commands,
) {
	for (entity, instance) in &instances {
		if instance.definition == BRAY && instance.stage == AttackStage::Execute {
			commands
				.entity(entity)
				.insert(BrayImpactArmed { due: params.bray.impact_delay.max(0.0) });
		}
	}
}

fn fire_bray_impact(
	instances: Query<(&AttackInstance, &BrayImpactArmed)>,
	mut fired: Local<Vec<AttackInstanceId>>,
	mut cues: MessageWriter<ImpactCue>,
) {
	fired.retain(|id| instances.iter().any(|(instance, _)| instance.id == *id));
	for (instance, armed) in &instances {
		if instance.stage != AttackStage::Execute || instance.stage_age + 1e-4 < armed.due {
			continue;
		}
		if fired.contains(&instance.id) {
			continue;
		}
		cues.write(ImpactCue {
			instance: instance.id,
			owner: instance.owner,
			origin: instance.snapshot.origin,
		});
		fired.push(instance.id);
	}
}

fn impact_to_spatial_rumble(
	mut impacts: MessageReader<ImpactCue>,
	mut rumbles: MessageWriter<SpatialRumbleCue>,
) {
	for impact in impacts.read() {
		rumbles.write(SpatialRumbleCue::heavy_landing(
			impact.instance,
			impact.owner,
			impact.origin,
		));
	}
}

pub fn is_bikanwarda_attack(id: AttackId) -> bool {
	id == PARADE || id == PARADE_BLAST || id == GOD_BEAM || id == BRAY
}
