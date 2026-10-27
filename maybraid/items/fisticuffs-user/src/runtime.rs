//! Equipment realize: hold / teardown the fisticuffs driver.

use bevy::prelude::*;
use character_items::InventoryItem;
use maybraid_equipment::{
	EquipmentSystems, EquippedItemHandler, PendingEquipmentHold, PendingEquipmentTeardown,
};

use crate::{drive, sensor, FisticuffsStrike, FisticuffsUser};

#[derive(Resource, Default)]
struct FisticuffsUserRuntime;

/// Attack machine + equipment realize. Gameplay assembly is the owner that
/// calls this; a second call is a no-op. No rendering or audio resources are
/// required. A host that installs physics also resolves the jab overlap.
pub fn register_fisticuffs_user_runtime(app: &mut App) {
	if app.world().contains_resource::<FisticuffsUserRuntime>() {
		return;
	}
	if !app.is_plugin_added::<damage::DamagePlugin>() {
		app.add_plugins(damage::DamagePlugin);
	}
	app.init_resource::<FisticuffsUserRuntime>()
		.add_message::<FisticuffsStrike>()
		.add_observer(drive::apply_attack_routing)
		.add_observer(drive::apply_attack_intents)
		.add_systems(
			Update,
			(
				drive::tick_attack_state,
				drive::start_pending_attacks,
				drive::latch_jab_aim,
				sensor::sync_fist_sensors,
			)
				.chain()
				.in_set(player::PlayerSystems::Intent),
		)
		.add_systems(
			PostUpdate,
			sensor::resolve_fist_hits
				.after(bevy::transform::TransformSystems::Propagate)
				.before(damage::DamageSystems::Apply),
		)
		.add_systems(
			Update,
			realize_equipment_teardown
				.in_set(EquipmentSystems::Teardown)
				.before(player::PlayerSystems::Intent),
		)
		.add_systems(Update, realize_equipment_hold.in_set(EquipmentSystems::Hold));
}

/// Cancel the in-flight jab and drop the driver. Sensors follow
/// [`RemovedComponents<FisticuffsUser>`] in [`crate::sync_fist_sensors`].
pub fn teardown_fisticuffs(commands: &mut Commands, user: Entity) {
	if let Ok(mut entity) = commands.get_entity(user) {
		entity.remove::<FisticuffsUser>();
	}
}

fn realize_equipment_teardown(
	mut commands: Commands,
	mut pending: Query<(Entity, Option<&mut FisticuffsUser>), With<PendingEquipmentTeardown>>,
) {
	for (entity, user) in &mut pending {
		if let Some(mut user) = user {
			user.state.cancel();
		}
		teardown_fisticuffs(&mut commands, entity);
	}
}

fn realize_equipment_hold(mut commands: Commands, pending: Query<(Entity, &PendingEquipmentHold)>) {
	for (entity, hold) in &pending {
		if !matches!(hold.inventory().primary_weapon(), Some(InventoryItem::Fisticuffs)) {
			continue;
		}
		if let Ok(mut entity) = commands.get_entity(entity) {
			entity.try_insert(FisticuffsUser::default());
			EquippedItemHandler::Fisticuffs.install(&mut entity);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AttackPhase, FisticuffsHitSensor};
	use bevy::time::TimeUpdateStrategy;
	use maybraid_character_controller::{ActorIntentEvent, CharacterIntent};
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};
	use std::time::Duration;

	#[derive(Resource, Default)]
	struct Strikes(usize);

	fn record_strikes(mut events: MessageReader<FisticuffsStrike>, mut strikes: ResMut<Strikes>) {
		strikes.0 += events.read().count();
	}

	#[test]
	fn runtime_advances_one_jab_without_presentation_and_registers_once() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(50)))
			.init_resource::<Strikes>();
		register_fisticuffs_user_runtime(&mut app);
		register_fisticuffs_user_runtime(&mut app);
		app.add_systems(PostUpdate, record_strikes);
		let actor = app
			.world_mut()
			.spawn((
				player::Player,
				FisticuffsUser::default(),
				EquippedItemHandler::Fisticuffs,
				Transform::IDENTITY,
			))
			.id();
		let other = app
			.world_mut()
			.spawn((player::Player, FisticuffsUser::default(), EquippedItemHandler::Fisticuffs))
			.id();
		app.init_resource::<InputRouter>();
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
		let input = app
			.world()
			.resource::<InputRouter>()
			.route_actor(InputSource::local_pad(), CharacterIntent::AttackRight)
			.ok_or_else(|| anyhow::anyhow!("actor route"))?;
		app.world_mut().trigger(ActorIntentEvent { entity: actor, input });
		app.update();
		for _ in 0..3 {
			app.update();
		}
		let user = app
			.world()
			.get::<FisticuffsUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("driver"))?;
		assert_eq!(user.state.phase(), Some(AttackPhase::Active));
		let other_user = app
			.world()
			.get::<FisticuffsUser>(other)
			.ok_or_else(|| anyhow::anyhow!("other driver"))?;
		assert!(!other_user.state.is_busy());
		assert_eq!(app.world().resource::<Strikes>().0, 1);
		assert_eq!(app.world_mut().query::<&FisticuffsHitSensor>().iter(app.world()).count(), 1);
		assert!(!app.is_plugin_added::<crate::FisticuffsUserPlugin>());
		assert!(!app.is_plugin_added::<maybraid_audio::AudioPlugin>());
		assert!(!app.world().contains_resource::<Assets<Mesh>>());
		for _ in 0..7 {
			app.update();
		}
		assert_eq!(app.world().resource::<Strikes>().0, 1);
		assert_eq!(app.world_mut().query::<&FisticuffsHitSensor>().iter(app.world()).count(), 0);
		Ok(())
	}
	#[test]
	fn addressed_jab_latches_chamber_aim_and_restarts_the_same_animation() -> anyhow::Result<()> {
		use crate::{AttackSpec, JabAim, Side};
		use characters::{AnimMailbox, CharacterHeading, CharacterRig, CharacterRoot, MemberOf};
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_resource::<InputRouter>();
		register_fisticuffs_user_runtime(&mut app);
		app.add_systems(
			Update,
			drive::apply_fisticuffs_visual.after(player::PlayerSystems::Intent),
		);
		let heading = Vec3::X;
		let look = player::PlayerLook {
			yaw: crate::body_camera_yaw(heading) + crate::AIM_STEP,
			pitch: crate::AIM_STEP,
			..default()
		};
		let actor = app
			.world_mut()
			.spawn((
				player::Player,
				look,
				FisticuffsUser::default(),
				EquippedItemHandler::Fisticuffs,
			))
			.id();
		let visual = app
			.world_mut()
			.spawn((CharacterRoot, CharacterHeading(heading), ChildOf(actor)))
			.id();
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		mailbox.posed = true;
		assert!(mailbox.settled());
		let body = app.world_mut().spawn((CharacterRig::default(), MemberOf(visual), mailbox)).id();
		let source = InputSource::virtual_pad(app.world_mut().spawn_empty().id());
		app.world_mut()
			.resource_mut::<InputRouter>()
			.assign_actor(source, ActorTarget::new(actor));
		for intent in [CharacterIntent::AttackLeft, CharacterIntent::AttackRight] {
			let input = app
				.world()
				.resource::<InputRouter>()
				.route_actor(source, intent)
				.ok_or_else(|| anyhow::anyhow!("actor route"))?;
			app.world_mut().trigger(ActorIntentEvent { entity: actor, input });
			app.update();
			let user = app
				.world()
				.get::<FisticuffsUser>(actor)
				.ok_or_else(|| anyhow::anyhow!("driver"))?;
			assert!(user.state.just_started());
			assert_eq!(user.state.aim(), Some(JabAim::from_look(&look, heading)));
			assert!(!app.world().get::<AnimMailbox>(body).is_some_and(AnimMailbox::settled));
			assert!(app.world().get::<characters::AnimRefRoot>(body).is_some());
			// Same Jab animation identity is retained for the next side, while the
			// state has reached recovery and the visible mailbox has settled.
			let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
			mailbox.posed = true;
			app.world_mut().entity_mut(body).insert(mailbox);
			let mut user = app
				.world_mut()
				.get_mut::<FisticuffsUser>(actor)
				.ok_or_else(|| anyhow::anyhow!("driver"))?;
			user.state.cancel();
			user.state.request(AttackSpec::jab(Side::Left));
			user.state.tick(0.15);
			user.state.tick(0.10);
		}
		Ok(())
	}
}
