//! Consume attack intents, tick state, emit the strike event, and pose the clip.

use bevy::prelude::*;
use character_animations::animations::{DEFAULT_BACKSWING, DEFAULT_JAB_TARGET};
use characters::{
	AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers,
	CharacterRig, CharacterRigRole, CharacterRoot, JabParams,
};
use maybraid_character_controller::CharacterIntent;
use maybraid_controllers_core::{InputRouter, InputSource};
use maybraid_equipment::EquippedItemHandler;
use player::{Player, PlayerLook, PlayerUse};

use crate::aim::JabAim;
use crate::attack::AttackSpec;
use crate::{FisticuffsStrike, FisticuffsUser};

pub(crate) fn apply_attack_intents(
	router: Option<Res<InputRouter>>,
	mut intents: MessageReader<CharacterIntent>,
	mut users: Query<
		(Entity, &mut FisticuffsUser, Option<&EquippedItemHandler>),
		(With<Player>, Without<PlayerUse>),
	>,
) {
	let mut requested = Vec::new();
	for intent in intents.read() {
		if let Some(spec) = AttackSpec::from_intent(*intent) {
			requested.push(spec);
		}
	}
	if requested.is_empty() {
		return;
	}
	let delivered = router
		.as_ref()
		.and_then(|router| router.delivered_actor(InputSource::local_pad()));
	for (entity, mut user, handler) in &mut users {
		if !handler.is_some_and(|handler| handler.is_fisticuffs()) {
			continue;
		}
		if router.is_some() && !delivered.is_some_and(|actor| actor.matches(entity)) {
			continue;
		}
		for spec in &requested {
			user.state.request(*spec);
		}
	}
}

/// Sample look into a quantized jab target when the punch chambers.
///
/// Latch is start-time ([`crate::attack::AttackState::just_started`]), not
/// Active, so the chamber already points at the cone and mid-punch look
/// does not wobble the fist.
pub(crate) fn latch_jab_aim(
	mut users: Query<(Entity, &mut FisticuffsUser, Option<&PlayerLook>), With<Player>>,
	visuals: Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
) {
	for (entity, mut user, look) in &mut users {
		if !user.state.just_started() {
			continue;
		}
		let heading = visuals
			.iter()
			.find(|(_, child_of)| child_of.parent() == entity)
			.map(|(heading, _)| heading.0)
			.unwrap_or(-Vec3::Z);
		let look = look.copied().unwrap_or_default();
		user.state.latch_aim(JabAim::from_look(&look, heading));
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
					combo_step: user.state.combo_step(),
					progress,
					target: user.state.aim().map(|aim| aim.target).unwrap_or(DEFAULT_JAB_TARGET),
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
	users: Query<&FisticuffsUser, (With<Player>, Without<PlayerUse>)>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	mut mailboxes: Query<&mut AnimMailbox>,
) {
	for (members, child_of) in &visuals {
		let Ok(user) = users.get(child_of.parent()) else {
			continue;
		};
		if !user.state.is_busy() {
			continue;
		}
		let Some(side) = user.state.side() else {
			continue;
		};
		let Some(progress) = user.state.clip_progress() else {
			continue;
		};
		let target = user.state.aim().map(|aim| aim.target).unwrap_or(DEFAULT_JAB_TARGET);
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
			if let Ok(mut entity) = commands.get_entity(member) {
				entity.try_insert((
					AnimRefRoot(AnimRef::new(AnimClip::Jab(JabParams {
						side,
						backswing: DEFAULT_BACKSWING,
						target,
					}))),
					AnimProgress(progress),
				));
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AttackSpec, AttackTiming, FisticuffsStrike, JAB_STARTUP_END};
	use character_rigs::Side;
	use maybraid_character_controller::CharacterIntent;

	#[test]
	fn intents_do_not_punch_without_the_driver() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<CharacterIntent>();
		let player = app.world_mut().spawn(Player).id();
		app.world_mut().write_message(CharacterIntent::AttackLeft);
		app.add_systems(Update, apply_attack_intents);
		app.update();
		assert!(app.world().get::<FisticuffsUser>(player).is_none());
	}

	#[test]
	fn intents_queue_only_while_fisticuffs_is_equipped() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<CharacterIntent>();
		let player = app
			.world_mut()
			.spawn((Player, FisticuffsUser::default(), EquippedItemHandler::Fisticuffs))
			.id();
		app.world_mut().write_message(CharacterIntent::AttackLeft);
		app.add_systems(Update, apply_attack_intents);
		app.update();
		let user = app.world().get::<FisticuffsUser>(player).expect("driver");
		assert!(user.state.is_busy());
		assert_eq!(user.state.side(), Some(Side::Left));
	}

	#[test]
	fn held_item_occupancy_blocks_jabs_even_with_the_driver() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<CharacterIntent>();
		let held = app.world_mut().spawn_empty().id();
		let player = app
			.world_mut()
			.spawn((Player, PlayerUse { driver: held }, FisticuffsUser::default()))
			.id();
		app.world_mut().write_message(CharacterIntent::AttackRight);
		app.add_systems(Update, apply_attack_intents);
		app.update();
		let user = app.world().get::<FisticuffsUser>(player).expect("driver");
		assert!(!user.state.is_busy());
	}

	#[test]
	fn cancel_before_tick_does_not_emit_a_strike() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<FisticuffsStrike>();
		let mut user = FisticuffsUser::default();
		user.state.request(AttackSpec::jab(Side::Left));
		user.state.tick(AttackTiming::JAB.duration_secs * JAB_STARTUP_END);
		assert!(user.state.just_entered_active() || user.state.is_busy());
		user.state.cancel();
		let player = app.world_mut().spawn((Player, user)).id();
		app.add_systems(Update, tick_attack_state);
		app.update();
		let user = app.world().get::<FisticuffsUser>(player).expect("driver");
		assert!(!user.state.just_entered_active());
		assert!(!user.state.is_busy());
	}
}
