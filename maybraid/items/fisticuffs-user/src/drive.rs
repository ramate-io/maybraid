//! Consume attack intents, tick state, emit the strike event, and pose the clip.

use bevy::prelude::*;
use character_animations::animations::{DEFAULT_BACKSWING, DEFAULT_JAB_TARGET};
use characters::{
	AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers,
	CharacterRig, CharacterRigRole, CharacterRoot, JabParams,
};
use maybraid_character_controller::{ActorControlStatus, ActorIntentEvent};
use maybraid_controllers_core::{ActorRoutingEvent, GestureEffect};
use maybraid_equipment::EquippedItemHandler;
use player::{Player, PlayerLook, PlayerUse};

use crate::aim::JabAim;
use crate::attack::AttackSpec;
use crate::{FisticuffsStrike, FisticuffsUser};

pub(crate) fn apply_attack_intents(
	event: On<ActorIntentEvent>,
	mut users: Query<
		(&mut FisticuffsUser, Option<&EquippedItemHandler>),
		(With<Player>, Without<PlayerUse>),
	>,
) {
	let Some(spec) = AttackSpec::from_intent(*event.input.payload()) else {
		return;
	};
	let Ok((mut user, handler)) = users.get_mut(event.entity) else {
		return;
	};
	if handler.is_some_and(|handler| handler.is_fisticuffs()) {
		user.pending_attacks.push(event.input.map(|_| spec));
	}
}

/// Start addressed requests after ticking, so chamber-time aim and pose restart
/// remain observable during this frame's realization.
pub(crate) fn start_pending_attacks(mut users: Query<&mut FisticuffsUser>, mut commands: Commands) {
	for mut user in &mut users {
		for input in std::mem::take(&mut user.pending_attacks) {
			input.queue(&mut commands, |world, input| {
				let Some(actor) = input.target().actor_entity() else {
					return;
				};
				let Ok(mut actor) = world.get_entity_mut(actor) else {
					return;
				};
				if actor.contains::<PlayerUse>()
					|| !actor
						.get::<EquippedItemHandler>()
						.is_some_and(|handler| handler.is_fisticuffs())
				{
					return;
				}
				if let Some(mut user) = actor.get_mut::<FisticuffsUser>() {
					user.state.request(input.into_payload());
				}
			});
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

/// Cancel only the actor that lost delivery. A newly assigned actor and NPC
/// attacks must not inherit the previous source owner's cancellation.
pub(crate) fn apply_attack_routing(
	event: On<ActorRoutingEvent>,
	mut users: Query<&mut FisticuffsUser>,
) {
	if event.event.gesture() != GestureEffect::Cancel
		|| !event.event.lost_actor().is_some_and(|actor| actor.matches(event.entity))
	{
		return;
	}
	if let Ok(mut user) = users.get_mut(event.entity) {
		user.state.cancel();
		user.pending_attacks.clear();
	}
}

pub(crate) fn tick_attack_state(
	time: Res<Time>,
	mut users: Query<(Entity, &mut FisticuffsUser, Option<&ActorControlStatus>)>,
	mut strikes: MessageWriter<FisticuffsStrike>,
) {
	let dt = time.delta_secs();
	for (attacker, mut user, control) in &mut users {
		if control.is_some_and(|control| control.is_frozen()) {
			continue;
		}
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
	use crate::{AttackTiming, JAB_STARTUP_END};
	use character_rigs::Side;
	use maybraid_character_controller::CharacterIntent;
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};

	fn send(app: &mut App, source: InputSource, intent: CharacterIntent) -> anyhow::Result<()> {
		let input = app
			.world()
			.resource::<InputRouter>()
			.route_actor(source, intent)
			.ok_or_else(|| anyhow::anyhow!("actor route"))?;
		let entity =
			input.target().actor_entity().ok_or_else(|| anyhow::anyhow!("actor target"))?;
		app.world_mut().trigger(ActorIntentEvent { entity, input });
		app.world_mut().flush();
		app.update();
		Ok(())
	}

	#[test]
	fn addressed_attacks_only_reach_the_equipped_actor() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<InputRouter>()
			.add_observer(apply_attack_intents)
			.add_systems(Update, start_pending_attacks);
		let first = app
			.world_mut()
			.spawn((Player, FisticuffsUser::default(), EquippedItemHandler::Fisticuffs))
			.id();
		let second = app
			.world_mut()
			.spawn((Player, FisticuffsUser::default(), EquippedItemHandler::Fisticuffs))
			.id();
		let a = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		let b = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		{
			let mut router = app.world_mut().resource_mut::<InputRouter>();
			router.assign_actor(a, ActorTarget::new(first));
			router.assign_actor(b, ActorTarget::new(second));
		}
		send(&mut app, a, CharacterIntent::AttackLeft)?;
		assert_eq!(
			app.world().get::<FisticuffsUser>(first).and_then(|user| user.state.side()),
			Some(Side::Left)
		);
		assert!(!app
			.world()
			.get::<FisticuffsUser>(second)
			.is_some_and(|user| user.state.is_busy()));
		send(&mut app, b, CharacterIntent::AttackRight)?;
		assert_eq!(
			app.world().get::<FisticuffsUser>(second).and_then(|user| user.state.side()),
			Some(Side::Right)
		);
		Ok(())
	}

	#[test]
	fn held_item_and_other_handlers_block_jabs() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<InputRouter>()
			.add_observer(apply_attack_intents)
			.add_systems(Update, start_pending_attacks);
		let held = app.world_mut().spawn_empty().id();
		let actor = app
			.world_mut()
			.spawn((
				Player,
				FisticuffsUser::default(),
				EquippedItemHandler::Fisticuffs,
				PlayerUse { driver: held },
			))
			.id();
		let source = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(source, ActorTarget::new(actor));
		send(&mut app, source, CharacterIntent::AttackRight)?;
		assert!(!app
			.world()
			.get::<FisticuffsUser>(actor)
			.is_some_and(|user| user.state.is_busy()));
		app.world_mut()
			.entity_mut(actor)
			.remove::<PlayerUse>()
			.insert(EquippedItemHandler::Firearm);
		send(&mut app, source, CharacterIntent::AttackLeft)?;
		assert!(!app
			.world()
			.get::<FisticuffsUser>(actor)
			.is_some_and(|user| user.state.is_busy()));
		app.world_mut().entity_mut(actor).remove::<FisticuffsUser>();
		send(&mut app, source, CharacterIntent::AttackLeft)?;
		assert!(app.world().get::<FisticuffsUser>(actor).is_none());
		Ok(())
	}

	#[test]
	fn queued_attack_is_rejected_after_its_binding_is_retired() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<InputRouter>()
			.add_observer(apply_attack_intents)
			.add_systems(Update, start_pending_attacks);
		let actor = app
			.world_mut()
			.spawn((Player, FisticuffsUser::default(), EquippedItemHandler::Fisticuffs))
			.id();
		let next = app.world_mut().spawn_empty().id();
		let source = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(source, ActorTarget::new(actor));
		let input = app
			.world()
			.resource::<InputRouter>()
			.route_actor(source, CharacterIntent::AttackRight)
			.ok_or_else(|| anyhow::anyhow!("route"))?;
		app.world_mut().trigger(ActorIntentEvent { entity: actor, input });
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(source, ActorTarget::new(next));
		app.update();
		assert!(!app
			.world()
			.get::<FisticuffsUser>(actor)
			.is_some_and(|user| user.state.is_busy()));
		Ok(())
	}

	#[test]
	fn cancel_before_tick_does_not_emit_a_strike() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_message::<FisticuffsStrike>();
		let mut user = FisticuffsUser::default();
		user.state.request(AttackSpec::jab(Side::Left));
		user.state.tick(AttackTiming::JAB.duration_secs * JAB_STARTUP_END);
		user.state.cancel();
		let actor = app.world_mut().spawn((Player, user)).id();
		app.add_systems(Update, tick_attack_state);
		app.update();
		let user = app
			.world()
			.get::<FisticuffsUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("driver"))?;
		assert!(!user.state.just_entered_active());
		assert!(!user.state.is_busy());
		Ok(())
	}
}
