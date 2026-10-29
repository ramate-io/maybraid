//! Realize addressed [`EquippedItemUse`] on the held gun, and hand recoil to the camera.

use bevy::prelude::*;
use firearms::{WeaponFired, WeaponTrigger};
use maybraid_controllers_core::{ActorRoutingEvent, RoutingKind};
use maybraid_equipment::EquippedItemUse;
use player_camera::{CameraRecoilHandoff, CameraTarget, FollowCamera};

use crate::weapon::RecoilPattern;
use crate::{FirearmUser, WeaponSwap};

/// Seconds to lerp from the current aim to the kicked aim. A new shot retargets
/// the remaining path over a fresh window.
pub(crate) const RECOIL_LERP_SECS: f32 = 0.08;

pub fn apply_fire_intents(
	use_item: On<EquippedItemUse>,
	users: Query<(&FirearmUser, Has<WeaponSwap>)>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	if !use_item.handler().is_firearm() {
		return;
	}
	let Ok((user, swapping)) = users.get(use_item.entity) else {
		return;
	};
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = use_item.held() && !swapping;
	}
}

/// Clear an actor's held trigger when delivery is blocked or ownership ends.
/// Suspension keeps the pad's held state for resume, while the trigger stays
/// neutral during the frozen interval.
pub fn apply_fire_routing(
	event: On<ActorRoutingEvent>,
	users: Query<&FirearmUser>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	if !event.event.applies_to(event.entity) {
		return;
	}
	match event.event.kind() {
		RoutingKind::Suspended
		| RoutingKind::Disconnected
		| RoutingKind::Removed
		| RoutingKind::Reassigned
		| RoutingKind::SourceChanged
		| RoutingKind::Captured => {}
		_ => return,
	}
	let Ok(user) = users.get(event.entity) else {
		return;
	};
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = false;
	}
}

pub(crate) fn queue_weapon_recoil(
	mut fired: MessageReader<WeaponFired>,
	users: Query<&FirearmUser>,
	mut patterns: Query<&mut RecoilPattern>,
) {
	for event in fired.read() {
		if event.recoil <= 0.0 {
			continue;
		}
		let Ok(user) = users.get(event.shooter) else {
			continue;
		};
		let Ok(mut pattern) = patterns.get_mut(user.held) else {
			continue;
		};
		pattern.shot = pattern.shot.wrapping_add(1);
		let kick = recoil_kick(pattern.seed, pattern.shot, event.recoil);
		pattern.remaining += kick;
		pattern.time_left = RECOIL_LERP_SECS;
	}
}

pub(crate) fn advance_weapon_recoil(
	time: Res<Time>,
	users: Query<(Entity, &FirearmUser)>,
	mut patterns: Query<&mut RecoilPattern>,
	followed: Query<&CameraTarget, With<FollowCamera>>,
	mut handoffs: MessageWriter<CameraRecoilHandoff>,
) {
	let dt = time.delta_secs();
	for (shooter, user) in &users {
		let Ok(mut pattern) = patterns.get_mut(user.held) else {
			continue;
		};
		let (step, remaining, time_left) = recoil_travel(pattern.remaining, pattern.time_left, dt);
		pattern.remaining = remaining;
		pattern.time_left = time_left;
		if step.length_squared() < 1e-12 {
			continue;
		}
		handoffs.write(CameraRecoilHandoff::new(
			shooter,
			step,
			followed.iter().any(|target| target.0 == Some(shooter)),
		));
	}
}

/// Linear step along `remaining` so the kick lands in `time_left` seconds.
pub(crate) fn recoil_travel(remaining: Vec2, time_left: f32, dt: f32) -> (Vec2, Vec2, f32) {
	if remaining.length_squared() < 1e-12 {
		return (Vec2::ZERO, Vec2::ZERO, 0.0);
	}
	if dt <= 0.0 {
		return (Vec2::ZERO, remaining, time_left.max(0.0));
	}
	if time_left <= dt {
		return (remaining, Vec2::ZERO, 0.0);
	}
	let step = remaining * (dt / time_left);
	(step, remaining - step, time_left - dt)
}

/// Strength-based pattern: yaw in `[-strength, strength]`, pitch in `[0, strength]`
/// (look up). Direction comes from a hash of the weapon seed and shot index.
pub(crate) fn recoil_kick(seed: u64, shot: u32, strength: f32) -> Vec2 {
	if strength <= 0.0 {
		return Vec2::ZERO;
	}
	let yaw = signed_unit(seed, shot, 1) * strength;
	let pitch = (signed_unit(seed, shot, 2) * 0.5 + 0.5) * strength;
	Vec2::new(yaw, pitch)
}

fn signed_unit(seed: u64, shot: u32, lane: u64) -> f32 {
	let hash = mix(mix(seed, u64::from(shot)), lane);
	let t = (hash >> 40) as u32 as f32 / ((1u32 << 24) as f32);
	t * 2.0 - 1.0
}

fn mix(seed: u64, value: u64) -> u64 {
	let mut hash = seed.wrapping_add(value).wrapping_add(0x9E37_79B9_7F4A_7C15);
	hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	hash = (hash ^ (hash >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	hash ^ (hash >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::time::TimeUpdateStrategy;
	use maybraid_controllers_core::{
		ActorTarget, CaptureKind, InputRouter, InputSource, RoutingKind, ScopeOwner, UiTarget,
	};
	use maybraid_equipment::{EquippedItemHandler, EquippedItemUse};
	use player::{CameraFollow, PlayerLook};
	use player_camera::{CameraController, CameraPov};

	const STRENGTH: f32 = 0.08;

	#[test]
	fn item_use_and_release_touch_only_the_addressed_trigger() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(apply_fire_intents);
		let first_gun = app.world_mut().spawn(WeaponTrigger(false)).id();
		let second_gun = app.world_mut().spawn(WeaponTrigger(true)).id();
		let first = app.world_mut().spawn(FirearmUser::holding(first_gun)).id();
		let second = app.world_mut().spawn(FirearmUser::holding(second_gun)).id();
		app.world_mut().trigger(EquippedItemUse::new(
			ActorTarget::new(first),
			EquippedItemHandler::Firearm,
			1.0,
			true,
		));
		assert_eq!(
			app.world().get::<WeaponTrigger>(first_gun).map(|trigger| trigger.0),
			Some(true)
		);
		assert_eq!(
			app.world().get::<WeaponTrigger>(second_gun).map(|trigger| trigger.0),
			Some(true)
		);
		app.world_mut().trigger(EquippedItemUse::new(
			ActorTarget::new(first),
			EquippedItemHandler::Firearm,
			0.0,
			false,
		));
		assert_eq!(
			app.world().get::<WeaponTrigger>(first_gun).map(|trigger| trigger.0),
			Some(false)
		);
		assert_eq!(
			app.world().get::<WeaponTrigger>(second_gun).map(|trigger| trigger.0),
			Some(true)
		);
		app.world_mut().trigger(EquippedItemUse::new(
			ActorTarget::new(second),
			EquippedItemHandler::Grenade,
			1.0,
			true,
		));
		assert_eq!(
			app.world().get::<WeaponTrigger>(second_gun).map(|trigger| trigger.0),
			Some(true)
		);
		Ok(())
	}

	#[test]
	fn captured_actor_clears_held_trigger() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(apply_fire_routing);
		let gun = app.world_mut().spawn(WeaponTrigger(true)).id();
		let shooter = app.world_mut().spawn(FirearmUser::holding(gun)).id();
		let mut router = InputRouter::default();
		router.assign_actor(InputSource::local_pad(), ActorTarget::new(shooter));
		router.acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(Entity::from_bits(3))),
			CaptureKind::Overlay,
		);
		let event = router
			.take_events()
			.into_iter()
			.find(|event| event.kind() == RoutingKind::Captured)
			.ok_or_else(|| anyhow::anyhow!("captured routing event"))?;
		app.world_mut().trigger(ActorRoutingEvent {
			entity: shooter,
			event,
			status: maybraid_controllers_core::ActorRouteStatus {
				source: InputSource::local_pad(),
				connected: true,
				captured: true,
				suspended: false,
				assigned: true,
			},
		});
		assert_eq!(app.world().get::<WeaponTrigger>(gun).map(|trigger| trigger.0), Some(false));
		Ok(())
	}

	#[test]
	fn kick_is_deterministic() {
		assert_eq!(recoil_kick(7, 3, STRENGTH), recoil_kick(7, 3, STRENGTH));
	}

	#[test]
	fn kick_scales_with_strength() {
		let unit = recoil_kick(7, 3, 1.0);
		let doubled = recoil_kick(7, 3, 2.0);
		assert!((doubled - unit * 2.0).length() < 1e-5);
	}

	#[test]
	fn kick_stays_in_strength_range() {
		for shot in 1..256 {
			let kick = recoil_kick(0xA11A_4A45_F1A4_0001, shot, STRENGTH);
			assert!(kick.x.abs() <= STRENGTH + 1e-5);
			assert!(kick.y >= -1e-5 && kick.y <= STRENGTH + 1e-5);
		}
	}

	#[test]
	fn sequential_shots_are_not_identical() {
		let first = recoil_kick(11, 1, STRENGTH);
		let second = recoil_kick(11, 2, STRENGTH);
		assert_ne!(first, second);
	}

	#[test]
	fn different_weapons_diverge() {
		let a: Vec<_> = (1..32).map(|shot| recoil_kick(1, shot, STRENGTH)).collect();
		let b: Vec<_> = (1..32).map(|shot| recoil_kick(2, shot, STRENGTH)).collect();
		assert_ne!(a, b);
	}

	#[test]
	fn travel_is_linear_in_time() {
		let remaining = Vec2::new(0.1, 0.2);
		let (first, mid, mid_t) =
			recoil_travel(remaining, RECOIL_LERP_SECS, RECOIL_LERP_SECS * 0.5);
		assert!((first - remaining * 0.5).length() < 1e-5);
		assert!((mid_t - RECOIL_LERP_SECS * 0.5).abs() < 1e-5);
		let (second, leftover, time_left) = recoil_travel(mid, mid_t, RECOIL_LERP_SECS);
		assert!((first + second - remaining).length() < 1e-5);
		assert!(leftover.length() < 1e-5);
		assert_eq!(time_left, 0.0);
	}

	#[test]
	fn travel_completes_when_dt_covers_the_window() {
		let remaining = Vec2::new(-0.04, 0.08);
		let (step, leftover, time_left) =
			recoil_travel(remaining, RECOIL_LERP_SECS, RECOIL_LERP_SECS);
		assert_eq!(step, remaining);
		assert_eq!(leftover, Vec2::ZERO);
		assert_eq!(time_left, 0.0);
	}

	fn recoil_app(seed: u64, dt: f32) -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(
				std::time::Duration::from_secs_f32(dt),
			))
			.add_message::<WeaponFired>()
			.add_message::<CameraRecoilHandoff>()
			.add_systems(
				Update,
				(queue_weapon_recoil, advance_weapon_recoil, player_camera::apply_recoil_handoff)
					.chain(),
			);
		let gun = app.world_mut().spawn(RecoilPattern::from_seed(seed)).id();
		let shooter = app
			.world_mut()
			.spawn((CameraFollow, FirearmUser::holding(gun), PlayerLook::default()))
			.id();
		let camera = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				FollowCamera::default(),
				CameraTarget(Some(shooter)),
				CameraController { pov: CameraPov::ThirdPerson, ..default() },
			))
			.id();
		app.update();
		(app, shooter, camera)
	}

	fn angles(app: &App, camera: Entity, look: Entity) -> (f32, f32, PlayerLook) {
		let camera = app.world().entity(camera).get::<CameraController>().unwrap();
		let look = *app.world().entity(look).get::<PlayerLook>().unwrap();
		(camera.yaw, camera.pitch, look)
	}

	#[test]
	fn followed_shot_lerps_camera_and_look() {
		let (mut app, shooter, camera) = recoil_app(42, RECOIL_LERP_SECS * 0.5);
		app.world_mut().write_message(WeaponFired::new(shooter, STRENGTH));
		app.update();
		let expected = recoil_kick(42, 1, STRENGTH) * 0.5;
		let (yaw, pitch, look) = angles(&app, camera, shooter);
		assert!((yaw - expected.x).abs() < 1e-5);
		assert!((pitch - expected.y).abs() < 1e-5);
		assert!((look.yaw - expected.x).abs() < 1e-5);
		assert!((look.pitch - expected.y).abs() < 1e-5);
		app.update();
		let expected = recoil_kick(42, 1, STRENGTH);
		let (yaw, pitch, look) = angles(&app, camera, shooter);
		assert!((yaw - expected.x).abs() < 1e-5);
		assert!((pitch - expected.y).abs() < 1e-5);
		assert!((look.yaw - expected.x).abs() < 1e-5);
		assert!((look.pitch - expected.y).abs() < 1e-5);
	}

	#[test]
	fn zero_recoil_does_not_kick() {
		let (mut app, shooter, camera) = recoil_app(42, RECOIL_LERP_SECS);
		app.world_mut().write_message(WeaponFired::new(shooter, 0.0));
		app.update();
		let (yaw, pitch, look) = angles(&app, camera, shooter);
		assert_eq!(yaw, 0.0);
		assert_eq!(pitch, 0.0);
		assert_eq!(look.yaw, 0.0);
		assert_eq!(look.pitch, 0.0);
	}

	#[test]
	fn npc_shot_does_not_kick_the_follow_camera() {
		let (mut app, player, camera) = recoil_app(42, RECOIL_LERP_SECS);
		let npc_gun = app.world_mut().spawn(RecoilPattern::from_seed(99)).id();
		let npc = app
			.world_mut()
			.spawn((FirearmUser::holding(npc_gun), PlayerLook::default()))
			.id();
		app.world_mut().write_message(WeaponFired::new(npc, STRENGTH));
		app.update();
		let expected = recoil_kick(99, 1, STRENGTH);
		let (yaw, pitch, player_look) = angles(&app, camera, player);
		assert_eq!(yaw, 0.0);
		assert_eq!(pitch, 0.0);
		assert_eq!(player_look.yaw, 0.0);
		let npc_look = app.world().entity(npc).get::<PlayerLook>().unwrap();
		assert!((npc_look.yaw - expected.x).abs() < 1e-5);
		assert!((npc_look.pitch - expected.y).abs() < 1e-5);
	}
}
