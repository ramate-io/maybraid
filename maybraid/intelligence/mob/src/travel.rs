//! Move the host (the tether) toward an active [`PoiGoal`].
//!
//! Journeying hosts with a [`RoutingIntelligenceUser`] slide along the current
//! corridor hop, including that hop's ground-snapped Y. Hosts without routing
//! keep a flat XZ step so a zero-Y POI does not bury them.

use bevy::prelude::*;
use poi_intelligence::PoiGoal;
use routing_intelligence::RoutingIntelligenceUser;

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
	mut hosts: Query<
		(&MobTravel, &PoiGoal, Option<&mut RoutingIntelligenceUser>, &mut Transform),
		With<Mob>,
	>,
) {
	let step_dt = time.delta_secs().max(0.0);
	for (travel, goal, routing, mut transform) in &mut hosts {
		let step = travel.speed * step_dt;
		let from = transform.translation;
		if let Some(mut routing) = routing {
			if routing.destination.is_some() {
				routing.advance(from);
				let hop = routing.current_hop(from).unwrap_or(goal.location.point);
				transform.translation = step_chord(from, hop, step);
				continue;
			}
		}
		transform.translation = step_xz(from, goal.location.point, step);
	}
}

/// Flat ground slide: preserve Y.
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

/// Lerp along a routing chord, including the hop's Y.
pub(crate) fn step_chord(from: Vec3, target: Vec3, step: f32) -> Vec3 {
	let delta = target - from;
	let distance = delta.length();
	if distance <= step.max(0.0) {
		return target;
	}
	if distance <= f32::EPSILON {
		return from;
	}
	from + delta * (step / distance)
}
