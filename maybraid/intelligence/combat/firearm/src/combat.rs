//! Firearm combat brain: who to shoot, how to aim, when to fire.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use crozon_characters::CharacterRoot;
use firearm_user::FirearmUser;
use firearms::{muzzle_world, BoneMap, FirearmMembers, RigRoot, WeaponTrigger};
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{MovementBody, MovementIntelligence};
use player::{PlayerLook, PlayerYawOwner};
use std::f32::consts::FRAC_PI_2;

use crate::target::{pick_target, FirearmObjective, SpottedTarget};

/// How a firearm combatant aims and stays on a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmIntelligenceSettings {
	/// 0..=1. 1 is a tight aim cone and a high fire threshold.
	pub accuracy: f32,
	/// 0..=1. Blend from center-mass toward the head.
	pub headshots: f32,
	/// 0..=1. Stick to the current target versus the nearest.
	pub focus: f32,
	/// 0..=1. Frequency of trigger opportunities once sufficiently aimed.
	pub trigger_happiness: f32,
	/// 0..=1. Willingness to fire through an obstructed line of fire.
	pub wall_firing: f32,
	/// Seconds a last-known observation remains actionable.
	pub target_spotting_memory: f32,
}

impl Default for FirearmIntelligenceSettings {
	fn default() -> Self {
		Self {
			accuracy: 0.75,
			headshots: 0.15,
			focus: 0.6,
			trigger_happiness: 0.45,
			wall_firing: 0.0,
			target_spotting_memory: 2.5,
		}
	}
}

/// Per-user firearm combat install. Fields [`FirearmObjective`].
#[derive(Component, Debug, Clone)]
pub struct FirearmIntelligence {
	pub objective: FirearmObjective,
	pub settings: FirearmIntelligenceSettings,
	pub(crate) engaged: Option<Entity>,
	aiming_head: bool,
	next_aim_choice_at: f32,
	next_trigger_at: f32,
}

impl FirearmIntelligence {
	pub fn new(objective: FirearmObjective) -> Self {
		Self {
			objective,
			settings: FirearmIntelligenceSettings::default(),
			engaged: None,
			aiming_head: false,
			next_aim_choice_at: 0.0,
			next_trigger_at: 0.0,
		}
	}
}

/// Select a remembered target and turn the user's desired look toward it.
///
/// Look is computed from the last posed muzzle when available. Aiming from the
/// eyes with a right-shoulder gun parallel-offsets shots past the target.
pub(crate) fn aim_at_firearm_targets(
	time: Res<Time>,
	mut combatants: Query<(
		Entity,
		&Transform,
		&MovementIntelligence,
		&FirearmUser,
		&mut FirearmIntelligence,
		&mut PlayerLook,
	)>,
	guns: Query<&FirearmMembers>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
) {
	let elapsed = time.elapsed_secs();
	for (entity, transform, movement, user, mut brain, mut look) in &mut combatants {
		let from = barrel_global(user.held, &guns, &maps, &globals)
			.map(muzzle_world)
			.map(|(muzzle, _)| muzzle)
			.unwrap_or_else(|| movement.ability.eye_point(transform.translation));
		let Some(target) =
			pick_target(from, &brain.objective.0, brain.engaged, brain.settings.focus).copied()
		else {
			brain.engaged = None;
			continue;
		};
		if brain.engaged != Some(target.entity) || elapsed >= brain.next_aim_choice_at {
			brain.aiming_head = frac_noise(
				entity.to_bits() as f32 * 0.013
					+ target.entity.to_bits() as f32 * 0.019
					+ elapsed.floor(),
			) < brain.settings.headshots.clamp(0.0, 1.0);
			brain.next_aim_choice_at = elapsed + 1.5;
		}
		brain.engaged = Some(target.entity);
		let headshots = if brain.aiming_head { 1.0 } else { 0.0 };
		let to = target.aim_point(headshots) - from;
		let (yaw, pitch) = look_angles(to, brain.settings.accuracy, entity, elapsed);
		look.yaw = yaw;
		look.pitch = pitch;
	}
}

/// Turn the visual body toward combat look before the held-firearm pose applies
/// its local yaw cone.
pub(crate) fn orient_firearm_combatants(
	time: Res<Time>,
	combatants: Query<(&PlayerLook, &FirearmIntelligence)>,
	mut visuals: Query<
		(&ChildOf, &mut Transform, Option<&mut PlayerYawOwner>),
		With<CharacterRoot>,
	>,
) {
	let amount = (time.delta_secs() * 5.0).clamp(0.0, 1.0);
	for (child_of, mut visual, yaw_owner) in &mut visuals {
		let Ok((look, brain)) = combatants.get(child_of.parent()) else {
			continue;
		};
		if brain.engaged.is_none() {
			if let Some(mut yaw_owner) = yaw_owner {
				*yaw_owner = PlayerYawOwner::Wish;
			}
			continue;
		}
		if let Some(mut yaw_owner) = yaw_owner {
			*yaw_owner = PlayerYawOwner::Look;
		}
		let forward = Quat::from_axis_angle(Vec3::Y, look.yaw) * -Vec3::Z;
		let mut target = *visual;
		target.look_to(-forward, Vec3::Y);
		visual.rotation = visual.rotation.slerp(target.rotation, amount);
	}
}

/// Pulse the held trigger only when the actual propagated firearm bore is
/// aligned and the current obstruction policy allows the shot.
pub(crate) fn fire_at_spotted_targets(
	spatial: SpatialQuery,
	time: Res<Time>,
	mut combatants: Query<(Entity, &mut FirearmIntelligence, &FirearmUser)>,
	guns: Query<&FirearmMembers>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	let now = time.elapsed_secs();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	for (entity, mut brain, user) in &mut combatants {
		let target = engaged_target(&brain).copied();
		let Some(target) = target else {
			set_trigger(user, false, &mut triggers);
			continue;
		};
		let Some(global) = barrel_global(user.held, &guns, &maps, &globals) else {
			set_trigger(user, false, &mut triggers);
			continue;
		};
		let (muzzle, bore) = muzzle_world(global);
		let headshots = if brain.aiming_head { 1.0 } else { 0.0 };
		let aim_at = target.aim_point(headshots);
		let delta = aim_at - muzzle;
		let distance = delta.length();
		if distance <= 1e-4 {
			set_trigger(user, false, &mut triggers);
			continue;
		}
		let desired = delta / distance;
		let aligned = bore.dot(desired)
			>= fire_alignment(brain.settings.accuracy, distance, target.capsule.radius);
		let Ok(shot) = Dir3::new(bore) else {
			set_trigger(user, false, &mut triggers);
			continue;
		};
		let blocked = spatial
			.cast_ray(muzzle, shot, distance, true, &filter)
			.is_some_and(|hit| hit.distance < distance - 0.05);
		let obstruction_allowed =
			!blocked || willing_to_fire_through_wall(entity, now, brain.settings.wall_firing);
		let happiness = brain.settings.trigger_happiness.clamp(0.0, 1.0);
		let ready = happiness > 0.0 && now >= brain.next_trigger_at;
		let fire = aligned && obstruction_allowed && ready;
		set_trigger(user, fire, &mut triggers);
		if fire {
			brain.next_trigger_at = now + trigger_interval(happiness);
		}
	}
}

fn engaged_target(brain: &FirearmIntelligence) -> Option<&SpottedTarget> {
	let engaged = brain.engaged?;
	brain.objective.0.iter().find(|target| target.entity == engaged)
}

fn barrel_global<'a>(
	held: Entity,
	guns: &Query<&FirearmMembers>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &'a Query<&GlobalTransform>,
) -> Option<&'a GlobalTransform> {
	let members = guns.get(held).ok()?;
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&barrel) = map.by_name.get("barrel") else {
			continue;
		};
		if let Ok(global) = globals.get(barrel) {
			return Some(global);
		}
	}
	None
}

fn set_trigger(user: &FirearmUser, fire: bool, triggers: &mut Query<&mut WeaponTrigger>) {
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = fire;
	}
}

fn trigger_interval(happiness: f32) -> f32 {
	1.8_f32.lerp(0.18, happiness.clamp(0.0, 1.0))
}

/// Cosine of the allowed bore error. Tightens with range so a passing shot can
/// actually hit the capsule; `accuracy` adds a little extra miss.
fn fire_alignment(accuracy: f32, distance: f32, radius: f32) -> f32 {
	let hit = (radius.max(0.05) / distance.max(0.2)).atan();
	let slack = (1.0 - accuracy.clamp(0.0, 1.0)) * 0.05 + 0.012;
	(hit + slack).clamp(0.01, 0.4).cos()
}

fn willing_to_fire_through_wall(entity: Entity, now: f32, willingness: f32) -> bool {
	let willingness = willingness.clamp(0.0, 1.0);
	if willingness <= 0.0 {
		return false;
	}
	if willingness >= 1.0 {
		return true;
	}
	frac_noise(entity.to_bits() as f32 * 0.017 + now.floor() * 7.13) < willingness
}

fn look_angles(to: Vec3, accuracy: f32, entity: Entity, elapsed: f32) -> (f32, f32) {
	let cone = (1.0 - accuracy.clamp(0.0, 1.0)) * 0.12;
	let shake = jitter(entity, elapsed) * cone;
	let yaw = (-to.x).atan2(-to.z) + shake.x;
	let xz = Vec2::new(to.x, to.z).length();
	let pitch = -to.y.atan2(xz.max(1e-4)) + shake.y;
	(yaw, pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1))
}

#[cfg(test)]
fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
	Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_rotation_x(-pitch) * -Vec3::Z
}

fn jitter(entity: Entity, elapsed: f32) -> Vec2 {
	let seed = entity.to_bits() as f32 * 0.013 + elapsed;
	Vec2::new(frac_noise(seed), frac_noise(seed * 1.37)) * 2.0 - Vec2::ONE
}

fn frac_noise(x: f32) -> f32 {
	(x.sin() * 43_758.547).fract().abs()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn look_dir_matches_look_angles_without_jitter() -> anyhow::Result<()> {
		let to = Vec3::new(3.0, 1.0, -4.0);
		let (yaw, pitch) = look_angles(to, 1.0, Entity::from_bits(1), 0.0);
		let aimed = look_dir(yaw, pitch);
		let expected = to.normalize();
		assert!(aimed.dot(expected) > 0.999, "{aimed} vs {expected}");
		Ok(())
	}

	#[test]
	fn trigger_happiness_shortens_the_interval() {
		assert!(trigger_interval(1.0) < trigger_interval(0.0));
	}

	#[test]
	fn fire_alignment_tightens_with_range() {
		let close = fire_alignment(1.0, 2.0, 0.4);
		let far = fire_alignment(1.0, 20.0, 0.4);
		assert!(far > close, "{far} vs {close}");
		assert!(far > 0.99, "{far}");
	}

	#[test]
	fn right_offset_muzzle_aims_left_of_eye() {
		let target = Vec3::new(0.0, 1.0, -10.0);
		let (eye_yaw, _) =
			look_angles(target - Vec3::new(0.0, 1.0, 0.0), 1.0, Entity::from_bits(1), 0.0);
		let (muzzle_yaw, _) =
			look_angles(target - Vec3::new(0.3, 1.0, 0.0), 1.0, Entity::from_bits(1), 0.0);
		assert!(muzzle_yaw > eye_yaw, "{muzzle_yaw} vs {eye_yaw}");
	}

	#[test]
	fn zero_wall_firing_rejects_obstructions() {
		assert!(!willing_to_fire_through_wall(Entity::from_bits(1), 0.0, 0.0));
		assert!(willing_to_fire_through_wall(Entity::from_bits(1), 0.0, 1.0));
	}
}
