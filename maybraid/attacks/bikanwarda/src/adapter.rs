//! Thin presentation adapter: [`attacks::AttackPresentationEvent`] → lane-local cues.
//!
//! Sibling crates subscribe without this crate depending on them — see cue types in
//! [`crate::cues`]. Gameplay timing and collision stay on the attacks runtime.

use attacks::{
	AttackPresentationEvent, AttackPresentationKind, AttackStage, AttackSystems, HazardShape,
	PresentationPhase,
};
use bevy::prelude::*;

use crate::cues::{
	BrayAnimationCue, BrayAnimationPhase, EyeGlowCue, GodBeamVfxCue, ParadeExplosionCue,
	SonicWaveVfxCue, SonicWaveVfxKind, SpatialRumbleCue,
};
use crate::lanes::{configure_presentation_sets, BikanwardaPresentationSystems};
use crate::{BikanwardaAttackParams, BRAY, GOD_BEAM, PARADE_BLAST};

pub fn presentation_adapter_plugin(app: &mut App) {
	configure_presentation_sets(app);
	app.add_systems(
		Update,
		adapt_attack_presentation
			.in_set(BikanwardaPresentationSystems::Adapt)
			.after(AttackSystems::Advance),
	);
}

fn adapt_attack_presentation(
	params: Res<BikanwardaAttackParams>,
	mut events: MessageReader<AttackPresentationEvent>,
	mut explosions: MessageWriter<ParadeExplosionCue>,
	mut god_beam: MessageWriter<GodBeamVfxCue>,
	mut sonic: MessageWriter<SonicWaveVfxCue>,
	mut rumbles: MessageWriter<SpatialRumbleCue>,
	mut glow: MessageWriter<EyeGlowCue>,
	mut bray_anim: MessageWriter<BrayAnimationCue>,
	mut sonic_spawned: Local<std::collections::HashSet<attacks::AttackInstanceId>>,
) {
	for event in events.read() {
		match &event.kind {
			AttackPresentationKind::Hazard { index, phase, pose, shape, wave } => {
				if event.definition == PARADE_BLAST && *phase == PresentationPhase::Spawned {
					explosions.write(ParadeExplosionCue {
						instance: event.instance,
						owner: event.owner,
						position: pose.origin,
						radius: params.parade.blast_radius,
					});
				}
				if event.definition == GOD_BEAM {
					if let HazardShape::Cylinder { radius, height } = shape {
						let diameter = radius.max(0.0) * 2.0;
						if matches!(
							phase,
							PresentationPhase::Spawned
								| PresentationPhase::Updated
								| PresentationPhase::Despawned
						) {
							god_beam.write(GodBeamVfxCue {
								instance: event.instance,
								owner: event.owner,
								phase: *phase,
								hazard_index: *index,
								ground: pose.origin,
								diameter,
								gameplay_height: height.max(0.0),
								active_secs: params.god_beam.execution.max(0.0),
							});
						}
					}
				}
				if event.definition == BRAY {
					match phase {
						PresentationPhase::Spawned => {
							if sonic_spawned.insert(event.instance) {
								let bray = params.bray;
								sonic.write(SonicWaveVfxCue {
									instance: event.instance,
									owner: event.owner,
									phase: *phase,
									hazard_index: *index,
									kind: SonicWaveVfxKind::Spawn {
										origin: pose.origin,
										range: bray.range.max(0.0),
										speed: bray.speed.max(0.0),
										radius: bray.radius.max(0.0),
										thickness: bray.half_depth.max(0.0) * 2.0,
										wave_count: bray.waves,
										interval: bray.interval.max(0.0),
										pitch: bray.pitch,
									},
								});
							}
							if let Some(wave) = wave {
								sonic.write(SonicWaveVfxCue {
									instance: event.instance,
									owner: event.owner,
									phase: *phase,
									hazard_index: *index,
									kind: SonicWaveVfxKind::Follow(*wave),
								});
								rumbles.write(SpatialRumbleCue::bray_wave(
									event.instance,
									event.owner,
									wave.origin,
									&params.bray,
								));
							}
						}
						PresentationPhase::Updated => {
							if let Some(wave) = wave {
								sonic.write(SonicWaveVfxCue {
									instance: event.instance,
									owner: event.owner,
									phase: *phase,
									hazard_index: *index,
									kind: SonicWaveVfxKind::Follow(*wave),
								});
							}
						}
						PresentationPhase::Despawned => {
							sonic_spawned.remove(&event.instance);
							sonic.write(SonicWaveVfxCue {
								instance: event.instance,
								owner: event.owner,
								phase: *phase,
								hazard_index: *index,
								kind: SonicWaveVfxKind::Despawn,
							});
						}
					}
				}
			}
			AttackPresentationKind::Telegraph { phase: PresentationPhase::Spawned, .. }
				if event.definition == BRAY =>
			{
				bray_anim.write(BrayAnimationCue {
					instance: event.instance,
					owner: event.owner,
					phase: BrayAnimationPhase::Windup,
				});
			}
			AttackPresentationKind::Stage { to, .. } => {
				if event.definition == GOD_BEAM && *to == AttackStage::Windup {
					glow.write(EyeGlowCue::god_beam_warning(event.instance, event.owner));
				}
				if event.definition == BRAY {
					let phase = match *to {
						AttackStage::Windup => BrayAnimationPhase::Windup,
						AttackStage::Execute => BrayAnimationPhase::Execute,
						AttackStage::Recover => BrayAnimationPhase::Recover,
					};
					bray_anim.write(BrayAnimationCue {
						instance: event.instance,
						owner: event.owner,
						phase,
					});
				}
			}
			AttackPresentationKind::Ended { .. } => {
				if event.definition == GOD_BEAM {
					glow.write(EyeGlowCue::off(event.instance, event.owner));
				}
				if event.definition == BRAY {
					sonic_spawned.remove(&event.instance);
					bray_anim.write(BrayAnimationCue {
						instance: event.instance,
						owner: event.owner,
						phase: BrayAnimationPhase::Cancelled,
					});
					sonic.write(SonicWaveVfxCue {
						instance: event.instance,
						owner: event.owner,
						phase: PresentationPhase::Despawned,
						hazard_index: 0,
						kind: SonicWaveVfxKind::Despawn,
					});
				}
			}
			_ => {}
		}
	}
}
