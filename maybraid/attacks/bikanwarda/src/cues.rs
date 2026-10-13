//! Presentation cues derived from attack events. Hosts map these to VFX, pad
//! rumble, and animation; they do not apply damage.

use attacks::{
	AttackId, AttackInstance, AttackInstanceId, AttackPresentationEvent, AttackPresentationKind,
	AttackStage, AttackSystems, PresentationPhase,
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

/// Distance-attenuated rumble / shake request. Not a pad pulse by itself.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SpatialRumbleCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub origin: Vec3,
	pub radius: f32,
	pub intensity: f32,
	pub duration: f32,
}

/// Separate landing / stomp cue. [#1119](https://github.com/ramate-io/maybraid/issues/1119)
/// asks that any heavy landing stay off the wave rumble channel.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ImpactCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub origin: Vec3,
}

/// Eye-glow on/off for God Beam. Sleep / cancel / despawn send `active: false`.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EyeGlowCue {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub active: bool,
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
		.add_systems(
			Update,
			(emit_presentation_cues, arm_bray_impact, fire_bray_impact)
				.chain()
				.after(AttackSystems::Advance),
		);
}

fn emit_presentation_cues(
	params: Res<BikanwardaAttackParams>,
	mut events: MessageReader<AttackPresentationEvent>,
	mut explosions: MessageWriter<ParadeExplosionCue>,
	mut rumbles: MessageWriter<SpatialRumbleCue>,
	mut glow: MessageWriter<EyeGlowCue>,
) {
	for event in events.read() {
		match event.kind {
			AttackPresentationKind::Hazard { phase: PresentationPhase::Spawned, pose, .. }
				if event.definition == PARADE_BLAST =>
			{
				explosions.write(ParadeExplosionCue {
					instance: event.instance,
					owner: event.owner,
					position: pose.origin,
					radius: params.parade.blast_radius,
				});
			}
			AttackPresentationKind::Hazard {
				phase: PresentationPhase::Spawned,
				pose,
				wave: Some(_),
				..
			} if event.definition == BRAY => {
				rumbles.write(SpatialRumbleCue {
					instance: event.instance,
					owner: event.owner,
					origin: pose.origin,
					radius: params.bray.rumble_radius,
					intensity: params.bray.rumble_intensity,
					duration: params.bray.rumble_secs,
				});
			}
			AttackPresentationKind::Stage { to: AttackStage::Windup, .. }
				if event.definition == GOD_BEAM =>
			{
				glow.write(EyeGlowCue {
					instance: event.instance,
					owner: event.owner,
					active: true,
				});
			}
			AttackPresentationKind::Ended { .. } if event.definition == GOD_BEAM => {
				glow.write(EyeGlowCue {
					instance: event.instance,
					owner: event.owner,
					active: false,
				});
			}
			_ => {}
		}
	}
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
	mut fired: Local<Vec<attacks::AttackInstanceId>>,
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

pub fn is_bikanwarda_attack(id: AttackId) -> bool {
	id == PARADE || id == PARADE_BLAST || id == GOD_BEAM || id == BRAY
}
