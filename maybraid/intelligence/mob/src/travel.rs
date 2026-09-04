//! Move the host (the tether) toward an active [`PoiGoal`].

use bevy::prelude::*;
use poi_intelligence::PoiGoal;

use crate::Mob;

/// Horizontal speed for relocating the pack tether.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MobTravel {
	pub speed: f32,
}

impl MobTravel {
	pub fn new(speed: f32) -> Self {
		Self { speed: speed.max(0.0) }
	}
}

impl Default for MobTravel {
	fn default() -> Self {
		Self::new(2.5)
	}
}

pub(crate) fn travel_mobs(
	time: Res<Time>,
	mut hosts: Query<(&MobTravel, &PoiGoal, &mut Transform), With<Mob>>,
) {
	let step_dt = time.delta_secs().max(0.0);
	for (travel, goal, mut transform) in &mut hosts {
		transform.translation =
			step_xz(transform.translation, goal.location.point, travel.speed * step_dt);
	}
}

pub(crate) fn step_xz(from: Vec3, target: Vec3, step: f32) -> Vec3 {
	let delta = Vec3::new(target.x - from.x, 0.0, target.z - from.z);
	let distance = delta.length();
	if distance <= step.max(0.0) {
		return Vec3::new(target.x, from.y, target.z);
	}
	if distance <= f32::EPSILON {
		return from;
	}
	from + delta * (step / distance)
}
