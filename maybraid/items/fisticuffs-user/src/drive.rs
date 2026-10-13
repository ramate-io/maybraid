//! Consume attack intents, tick state, emit the strike event, and pose the clip.

use bevy::prelude::*;
use character_animations::animations::{DEFAULT_BACKSWING, DEFAULT_JAB_TARGET};
use characters::{
	AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterMembers, CharacterRig,
	CharacterRigRole, CharacterRoot, JabParams,
};
use maybraid_character_controller::CharacterIntent;
use player::{Player, PlayerUse};

use crate::attack::AttackSpec;
use crate::{FisticuffsStrike, FisticuffsUser};

pub(crate) fn ensure_fisticuffs_user(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, Without<FisticuffsUser>)>,
) {
	for entity in &players {
		commands.entity(entity).insert(FisticuffsUser::default());
	}
}

pub(crate) fn apply_attack_intents(
	mut intents: MessageReader<CharacterIntent>,
	mut users: Query<(&mut FisticuffsUser, Has<PlayerUse>), With<Player>>,
) {
	let mut requested = Vec::new();
	for intent in intents.read() {
		if let Some(spec) = AttackSpec::from_intent(*intent) {
			requested.push(spec);
		}
	}
	for (mut user, occupied) in &mut users {
		if occupied {
			if user.state.is_busy() {
				user.state.cancel();
			}
			continue;
		}
		for spec in &requested {
			user.state.request(*spec);
		}
	}
}

pub(crate) fn tick_attack_state(
	time: Res<Time>,
	mut users: Query<(Entity, &mut FisticuffsUser)>,
	mut strikes: MessageWriter<FisticuffsStrike>,
) {
	let dt = time.delta_secs();
	for (attacker, mut user) in &mut users {
		user.state.tick(dt);
		if user.state.just_entered_active() {
			if let (Some(spec), Some(progress)) = (user.state.spec(), user.state.clip_progress()) {
				strikes.write(FisticuffsStrike {
					attacker,
					side: spec.side,
					kind: spec.kind,
					progress,
				});
			}
		}
	}
}

/// Replace the locomotion clip while a jab is playing. Full-body on purpose:
/// `Jab` already includes lead-leg stance and trunk. The capsule motor is
/// untouched. Same-`AnimId` restarts blend from the visible mailbox pose.
pub(crate) fn apply_fisticuffs_visual(
	mut commands: Commands,
	users: Query<(&FisticuffsUser, Has<PlayerUse>), With<Player>>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	mut mailboxes: Query<&mut AnimMailbox>,
) {
	for (members, child_of) in &visuals {
		let Ok((user, occupied)) = users.get(child_of.parent()) else {
			continue;
		};
		if occupied || !user.state.is_busy() {
			continue;
		}
		let Some(side) = user.state.side() else {
			continue;
		};
		let Some(progress) = user.state.clip_progress() else {
			continue;
		};
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			if user.state.just_started() {
				if let Ok(mut mailbox) = mailboxes.get_mut(member) {
					mailbox.restart_from_visible();
				}
			}
			commands.entity(member).insert((
				AnimRefRoot(AnimRef::new(AnimClip::Jab(JabParams {
					side,
					backswing: DEFAULT_BACKSWING,
					target: DEFAULT_JAB_TARGET,
				}))),
				AnimProgress(progress),
			));
		}
	}
}
