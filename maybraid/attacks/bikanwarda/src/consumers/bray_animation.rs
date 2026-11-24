//! [`BrayAnimationCue`] → [`character_motion::AnimClip::epiphant_bray`].
//!
//! Cues name the gameplay actor. The clip host is the typed body rig bound by
//! [`character_motion::ActorBodyRig`] — not the actor itself after an
//! actor/visual split.

use std::collections::HashMap;

use attacks::AttackInstanceId;
use bevy::prelude::*;
use character_motion::{ActorBodyRig, AnimClip, AnimRef, AnimRefRoot};

use crate::consumers::BikanwardaPresentationSystems;
use crate::{BrayAnimationCue, BrayAnimationPhase};

#[derive(Clone, Debug)]
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
	bindings: Query<&ActorBodyRig>,
	mut clips: Query<&mut AnimRefRoot>,
	mut overrides: Local<HashMap<Entity, BrayClipOverride>>,
) {
	for cue in cues.read() {
		let Ok(binding) = bindings.get(cue.owner) else {
			continue;
		};
		let rig = binding.rig;
		match cue.phase {
			BrayAnimationPhase::Windup | BrayAnimationPhase::Execute => {
				if overrides.contains_key(&rig) {
					continue;
				}
				let Ok(mut anim) = clips.get_mut(rig) else {
					continue;
				};
				overrides.insert(rig, BrayClipOverride { instance: cue.instance, restore: anim.0 });
				anim.0 = AnimRef::new(AnimClip::epiphant_bray());
			}
			BrayAnimationPhase::Recover | BrayAnimationPhase::Cancelled => {
				let Some(override_) = overrides.get(&rig) else {
					continue;
				};
				if override_.instance != cue.instance {
					continue;
				}
				let restore = override_.restore;
				if let Ok(mut anim) = clips.get_mut(rig) {
					anim.0 = restore;
				}
				overrides.remove(&rig);
			}
		}
	}
}
