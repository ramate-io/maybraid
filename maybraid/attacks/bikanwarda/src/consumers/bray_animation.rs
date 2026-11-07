//! [`BrayAnimationCue`] → [`character_motion::AnimClip::epiphant_bray`].

use attacks::AttackInstanceId;
use bevy::prelude::*;
use character_motion::{AnimClip, AnimRef, AnimRefRoot};

use crate::consumers::BikanwardaPresentationSystems;
use crate::{BrayAnimationCue, BrayAnimationPhase};

#[derive(Component, Clone, Debug)]
struct BrayClipOverride {
	instance: AttackInstanceId,
	restore: AnimRef,
}

pub fn plugin(app: &mut App) {
	app.add_systems(
		Update,
		consume_bray_animation_cues.in_set(BikanwardaPresentationSystems::Consume),
	);
}

fn consume_bray_animation_cues(
	mut cues: MessageReader<BrayAnimationCue>,
	mut fresh: Query<(Entity, &mut AnimRefRoot), Without<BrayClipOverride>>,
	mut restoring: Query<(&mut AnimRefRoot, &BrayClipOverride)>,
	mut commands: Commands,
) {
	for cue in cues.read() {
		match cue.phase {
			BrayAnimationPhase::Windup | BrayAnimationPhase::Execute => {
				if let Ok((entity, mut anim)) = fresh.get_mut(cue.owner) {
					commands.entity(entity).insert(BrayClipOverride {
						instance: cue.instance,
						restore: anim.0.clone(),
					});
					anim.0 = AnimRef::new(AnimClip::epiphant_bray());
				}
			}
			BrayAnimationPhase::Recover => {}
			BrayAnimationPhase::Cancelled => {
				if let Ok((mut anim, override_)) = restoring.get_mut(cue.owner) {
					if override_.instance == cue.instance {
						anim.0 = override_.restore.clone();
						commands.entity(cue.owner).remove::<BrayClipOverride>();
					}
				}
			}
		}
	}
}
