//! Sightline-backed target observation and memory.

use avian3d::prelude::{LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{MovementBody, MovementIntelligence};

use crate::combat::FirearmIntelligence;
use crate::movement::FirearmMovementIntelligence;
use crate::target::{retain_recent, upsert_observation, FirearmSpotting, SpottedTarget};

pub(crate) fn spot_firearm_targets(
	spatial: SpatialQuery,
	time: Res<Time>,
	mut spotters: Query<(
		&Transform,
		&MovementIntelligence,
		&FirearmSpotting,
		&mut FirearmIntelligence,
		&mut FirearmMovementIntelligence,
	)>,
	targets: Query<(&Transform, Option<&LinearVelocity>)>,
) {
	let now = time.elapsed_secs();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	for (transform, movement, spotting, mut combat, mut combat_movement) in &mut spotters {
		let memory = combat.settings.target_spotting_memory.max(0.0);
		retain_recent(&mut combat.objective.0, now, memory);

		let observer = movement.ability.eye_point(transform.translation);
		for candidate in &spotting.candidates {
			let Ok((target, velocity)) = targets.get(candidate.entity) else {
				continue;
			};
			let position = target.translation;
			if !can_see_capsule(observer, position, candidate.capsule, &spatial, &filter) {
				continue;
			}
			upsert_observation(
				&mut combat.objective.0,
				SpottedTarget {
					entity: candidate.entity,
					position,
					capsule: candidate.capsule,
					movement_vector: velocity.map_or(Vec3::ZERO, |velocity| velocity.0),
					spotted_at: now,
				},
			);
		}
		combat_movement.objective.0.clone_from(&combat.objective.0);
	}
}

fn can_see_capsule(
	observer: Vec3,
	position: Vec3,
	capsule: crate::TargetCapsule,
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
) -> bool {
	[
		capsule.center_mass(position),
		capsule.head(position),
		position - Vec3::Y * (capsule.half_height * 0.5),
	]
	.into_iter()
	.any(|point| clear_segment(observer, point, spatial, filter))
}

pub(crate) fn clear_segment(
	start: Vec3,
	end: Vec3,
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
) -> bool {
	let delta = end - start;
	let distance = delta.length();
	if distance <= 1e-4 {
		return true;
	}
	let Ok(direction) = Dir3::new(delta) else {
		return true;
	};
	spatial
		.cast_ray(start, direction, distance, true, filter)
		.is_none_or(|hit| hit.distance >= distance - 0.05)
}
