//! Firearm combat brain: who to shoot, how to aim, when to fire.

use bevy::prelude::*;
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use player::PlayerLook;
use std::f32::consts::FRAC_PI_2;

use crate::target::{pick_target, FirearmObjective};

/// How a firearm combatant aims and stays on a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmIntelligenceSettings {
	/// 0..=1. 1 is a tight aim cone and a high fire threshold.
	pub accuracy: f32,
	/// 0..=1. Blend from center-mass toward the head.
	pub headshots: f32,
	/// 0..=1. Stick to the current target versus the nearest.
	pub focus: f32,
}

impl Default for FirearmIntelligenceSettings {
	fn default() -> Self {
		Self { accuracy: 0.75, headshots: 0.15, focus: 0.6 }
	}
}

/// Per-user firearm combat install. Fields [`FirearmObjective`].
#[derive(Component, Debug, Clone)]
pub struct FirearmIntelligence {
	pub objective: FirearmObjective,
	pub settings: FirearmIntelligenceSettings,
	engaged: Option<Entity>,
}

impl FirearmIntelligence {
	pub fn new(objective: FirearmObjective) -> Self {
		Self { objective, settings: FirearmIntelligenceSettings::default(), engaged: None }
	}

	pub fn aim_height(&self, feet_y: f32, hip_height: f32, eye_height: f32) -> f32 {
		let mass = feet_y + hip_height;
		let head = feet_y + eye_height;
		mass.lerp(head, self.settings.headshots.clamp(0.0, 1.0))
	}
}

const HIP_HEIGHT: f32 = 0.55;
const EYE_HEIGHT: f32 = 1.45;
const FEET_BELOW_ORIGIN: f32 = 0.9;

pub(crate) fn engage_firearm_targets(
	time: Res<Time>,
	mut combatants: Query<(Entity, &mut FirearmIntelligence, &mut PlayerLook, &FirearmUser)>,
	transforms: Query<&Transform>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	let elapsed = time.elapsed_secs();
	for (entity, mut brain, mut look, user) in &mut combatants {
		let Ok(from_tf) = transforms.get(entity) else {
			continue;
		};
		let from = from_tf.translation;
		let picked = pick_target(
			from,
			&brain.objective.0,
			|target| transforms.get(target).ok().map(|tf| tf.translation),
			brain.engaged,
			brain.settings.focus,
		);
		brain.engaged = picked;
		let Some(target) = picked else {
			if let Ok(mut trigger) = triggers.get_mut(user.held) {
				trigger.0 = false;
			}
			continue;
		};
		let Ok(target_tf) = transforms.get(target) else {
			continue;
		};
		let feet_y = target_tf.translation.y - FEET_BELOW_ORIGIN;
		let aim_at = Vec3::new(
			target_tf.translation.x,
			brain.aim_height(feet_y, HIP_HEIGHT, EYE_HEIGHT),
			target_tf.translation.z,
		);
		let to = aim_at - from;
		let (yaw, pitch) = look_angles(to, brain.settings.accuracy, entity, elapsed);
		look.yaw = yaw;
		look.pitch = pitch;
		let aimed = look_dir(yaw, pitch);
		let desired = to.normalize_or_zero();
		let threshold = 0.85_f32.lerp(0.995, brain.settings.accuracy.clamp(0.0, 1.0));
		if let Ok(mut trigger) = triggers.get_mut(user.held) {
			trigger.0 = desired.dot(aimed) >= threshold;
		}
	}
}

fn look_angles(to: Vec3, accuracy: f32, entity: Entity, elapsed: f32) -> (f32, f32) {
	let cone = (1.0 - accuracy.clamp(0.0, 1.0)) * 0.12;
	let shake = jitter(entity, elapsed) * cone;
	let yaw = (-to.x).atan2(-to.z) + shake.x;
	let xz = Vec2::new(to.x, to.z).length();
	let pitch = -to.y.atan2(xz.max(1e-4)) + shake.y;
	(yaw, pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1))
}

fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
	Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_rotation_x(-pitch) * -Vec3::Z
}

fn jitter(entity: Entity, elapsed: f32) -> Vec2 {
	let seed = entity.to_bits() as f32 * 0.013 + elapsed;
	Vec2::new(frac_noise(seed), frac_noise(seed * 1.37)) * 2.0 - Vec2::ONE
}

fn frac_noise(x: f32) -> f32 {
	(x.sin() * 43758.5453).fract().abs()
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
	fn aim_height_lerps_toward_the_head() {
		let mut brain = FirearmIntelligence::new(FirearmObjective::default());
		brain.settings.headshots = 0.0;
		assert!((brain.aim_height(0.0, 0.55, 1.45) - 0.55).abs() < 1e-4);
		brain.settings.headshots = 1.0;
		assert!((brain.aim_height(0.0, 0.55, 1.45) - 1.45).abs() < 1e-4);
	}
}
