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

use crate::los::clear_segment;
use crate::target::{live_aim_point, pick_target, FirearmObjective, SpottedTarget};

/// How a firearm combatant aims and stays on a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmIntelligenceSettings {
	/// 0..=1. 1 is a tight aim cone and a high fire threshold.
	pub accuracy: f32,
	/// Prefer the remembered visible head sample when the shooter is going for one.
	pub headshots: f32,
	/// Sightline rays traced each frame, shared across ranked candidates.
	pub vision: u16,
	/// 0..=1. Stick to the current target versus the nearest. Also spends more
	/// of [`Self::vision`] on the highest-ranked target.
	pub focus: f32,
	/// 0..=1. How quickly they pull the trigger once the bore is on target.
	/// The weapon interval is the rate of fire after that; this is not a second
	/// cadence. 0 never fires. 1 fires as soon as aligned.
	pub trigger_happiness: f32,
	/// 0..=1. Willingness to fire through an obstructed line of fire.
	pub wall_firing: f32,
	/// Seconds a last-known observation remains actionable for look and hunt.
	pub target_spotting_memory: f32,
	/// Seconds since the last clear sightline required to actually fire.
	/// Look may track a remembered pose; fire needs a current hole to shoot through.
	pub fire_spotting_freshness: f32,
}

impl Default for FirearmIntelligenceSettings {
	fn default() -> Self {
		Self {
			accuracy: 0.75,
			headshots: 0.15,
			vision: 9,
			focus: 0.6,
			trigger_happiness: 0.45,
			wall_firing: 0.0,
			target_spotting_memory: 2.5,
			fire_spotting_freshness: 0.2,
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
	on_target: bool,
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
			on_target: false,
		}
	}

	pub fn has_fresh_sight(&self, entity: Entity, now: f32) -> bool {
		self.objective.0.iter().any(|target| {
			target.entity == entity && target.is_fresh(now, self.settings.fire_spotting_freshness)
		})
	}
}

/// Select a remembered target and turn the user's desired look toward it.
///
/// Look is solved from the stock (the pose pivot), not the muzzle. The gun
/// rotates about the shoulder; aiming from the barrel tip pitches the bore high.
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
	bodies: Query<&Transform, Without<FirearmIntelligence>>,
) {
	let elapsed = time.elapsed_secs();
	for (entity, transform, movement, user, mut brain, mut look) in &mut combatants {
		let from = aim_pivot(user.held, transform.translation, movement, &guns, &maps, &globals);
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
		let current = bodies.get(target.entity).ok().map(|transform| transform.translation);
		let to = live_aim_point(target, headshots, current) - from;
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

/// Hold the trigger when the posed bore is on a freshly spotted point and the
/// obstruction policy allows the shot.
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
			brain.on_target = false;
			set_trigger(user, false, &mut triggers);
			continue;
		};
		let Some(global) = gun_landmark(user.held, "barrel", &guns, &maps, &globals) else {
			brain.on_target = false;
			set_trigger(user, false, &mut triggers);
			continue;
		};
		let (muzzle, bore) = muzzle_world(global);
		let fresh = target.is_fresh(now, brain.settings.fire_spotting_freshness);
		let headshots = if brain.aiming_head { 1.0 } else { 0.0 };
		let aim_at = target.aim_point(headshots);
		let center = target.capsule.center_mass(target.position);
		let delta = center - muzzle;
		let distance = delta.length();
		if !fresh || distance <= 1e-4 {
			brain.on_target = false;
			set_trigger(user, false, &mut triggers);
			continue;
		}
		let desired = delta / distance;
		let aligned = bore.dot(desired)
			>= fire_alignment(brain.settings.accuracy, distance, target.capsule.radius);
		let blocked = !clear_segment(muzzle, aim_at, &spatial, &filter);
		let obstruction_allowed =
			!blocked || willing_to_fire_through_wall(entity, now, brain.settings.wall_firing);
		if !hold_trigger(
			aligned,
			obstruction_allowed,
			brain.on_target,
			brain.settings.trigger_happiness,
		) {
			brain.on_target = false;
			set_trigger(user, false, &mut triggers);
			continue;
		}
		if !brain.on_target {
			brain.on_target = true;
			brain.next_trigger_at = now + acquire_delay(brain.settings.trigger_happiness);
		}
		set_trigger(user, now >= brain.next_trigger_at, &mut triggers);
	}
}

fn engaged_target(brain: &FirearmIntelligence) -> Option<&SpottedTarget> {
	let engaged = brain.engaged?;
	brain.objective.0.iter().find(|target| target.entity == engaged)
}

fn gun_landmark<'a>(
	held: Entity,
	name: &str,
	guns: &Query<&FirearmMembers>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &'a Query<&GlobalTransform>,
) -> Option<&'a GlobalTransform> {
	let members = guns.get(held).ok()?;
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(global);
		}
	}
	None
}

fn aim_pivot(
	held: Entity,
	body: Vec3,
	movement: &MovementIntelligence,
	guns: &Query<&FirearmMembers>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &Query<&GlobalTransform>,
) -> Vec3 {
	if let Some(stock) = gun_landmark(held, "stock", guns, maps, globals) {
		return stock.translation();
	}
	if let Ok(root) = globals.get(held) {
		return root.translation();
	}
	movement.ability.eye_point(body)
}

fn set_trigger(user: &FirearmUser, fire: bool, triggers: &mut Query<&mut WeaponTrigger>) {
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = fire;
	}
}

fn acquire_delay(happiness: f32) -> f32 {
	0.45 * (1.0 - happiness.clamp(0.0, 1.0))
}

/// First shot needs a bore on the capsule. After that, keep holding through a
/// frame of jitter so a lock is not dropped the way a fresh respawn never is.
fn hold_trigger(aligned: bool, obstruction_allowed: bool, on_target: bool, happiness: f32) -> bool {
	if happiness <= 0.0 || !obstruction_allowed {
		return false;
	}
	aligned || on_target
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
	let pitch = to.y.atan2(xz.max(1e-4)) + shake.y;
	(yaw, pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1))
}

#[cfg(test)]
fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
	Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_rotation_x(pitch) * -Vec3::Z
}

fn jitter(entity: Entity, elapsed: f32) -> Vec2 {
	let seed = entity.to_bits() as f32 * 0.013 + elapsed.floor();
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
	fn trigger_happiness_shortens_acquire_delay() {
		assert!(acquire_delay(1.0) < acquire_delay(0.0));
		assert!(acquire_delay(1.0) < 1e-4);
	}

	#[test]
	fn fire_alignment_tightens_with_range() {
		let close = fire_alignment(1.0, 2.0, 0.4);
		let far = fire_alignment(1.0, 20.0, 0.4);
		assert!(far > close, "{far} vs {close}");
		assert!(far > 0.99, "{far}");
	}

	#[test]
	fn right_offset_pivot_aims_left_of_eye() {
		let target = Vec3::new(0.0, 1.0, -10.0);
		let (eye_yaw, _) =
			look_angles(target - Vec3::new(0.0, 1.0, 0.0), 1.0, Entity::from_bits(1), 0.0);
		let (stock_yaw, _) =
			look_angles(target - Vec3::new(0.3, 1.0, 0.0), 1.0, Entity::from_bits(1), 0.0);
		assert!(stock_yaw > eye_yaw, "{stock_yaw} vs {eye_yaw}");
	}

	#[test]
	fn raised_muzzle_is_not_the_pose_pivot() {
		let target = Vec3::new(0.0, 1.05, -8.0);
		let stock = Vec3::new(0.25, 1.35, 0.0);
		let muzzle = Vec3::new(0.25, 1.55, -0.55);
		let (stock_yaw, stock_pitch) = look_angles(target - stock, 1.0, Entity::from_bits(1), 0.0);
		let (muzzle_yaw, muzzle_pitch) =
			look_angles(target - muzzle, 1.0, Entity::from_bits(1), 0.0);
		let stock_dir = look_dir(stock_yaw, stock_pitch);
		let muzzle_dir = look_dir(muzzle_yaw, muzzle_pitch);
		assert!(
			(stock_dir - muzzle_dir).length() > 0.01,
			"stock {stock_dir} vs muzzle {muzzle_dir}"
		);
	}

	#[test]
	fn zero_wall_firing_rejects_obstructions() {
		assert!(!willing_to_fire_through_wall(Entity::from_bits(1), 0.0, 0.0));
		assert!(willing_to_fire_through_wall(Entity::from_bits(1), 0.0, 1.0));
	}

	#[test]
	fn hold_trigger_keeps_a_lock_through_a_missed_alignment() {
		assert!(hold_trigger(false, true, true, 0.9));
		assert!(!hold_trigger(false, true, false, 0.9));
		assert!(!hold_trigger(true, false, true, 0.9));
		assert!(!hold_trigger(true, true, false, 0.0));
	}

	#[test]
	fn fresh_sight_requires_a_recent_combat_observation() {
		let mut brain = FirearmIntelligence::new(FirearmObjective::default());
		let entity = Entity::from_bits(1);
		assert!(!brain.has_fresh_sight(entity, 1.0));
		brain.objective.0.push(SpottedTarget {
			entity,
			position: Vec3::ZERO,
			capsule: crate::target::TargetCapsule::new(0.4, 0.9),
			visible: Vec3::ZERO,
			visible_head: None,
			movement_vector: Vec3::ZERO,
			spotted_at: 0.9,
		});
		assert!(brain.has_fresh_sight(entity, 1.0));
		assert!(!brain.has_fresh_sight(entity, 1.5));
	}
}
